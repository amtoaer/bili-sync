use core::str;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use futures::{StreamExt, TryStreamExt, future};
use reqwest::{Method, header::{RANGE, CONTENT_LENGTH}};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncSeekExt};
use std::io::SeekFrom;
use tokio_util::io::StreamReader;
use tracing::{warn, error, info};

use crate::bilibili::Client;
pub struct Downloader {
    client: Client,
}

impl Downloader {
    // Downloader 使用带有默认 Header 的 Client 构建
    // 拿到 url 后下载文件不需要任何 cookie 作为身份凭证
    // 但如果不设置默认 Header，下载时会遇到 403 Forbidden 错误
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, url: &str, path: &Path) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        let mut file = match File::create(path).await {
            Ok(f) => f,
            Err(e) => {
                error!("创建文件失败: {:#}", e);
                return Err(e.into());
            }
        };
        
        let resp = match self.client.request(Method::GET, url, None).send().await {
            Ok(r) => match r.error_for_status() {
                Ok(r) => r,
                Err(e) => {
                    error!("HTTP状态码错误: {:#}", e);
                    return Err(e.into());
                }
            },
            Err(e) => {
                error!("HTTP请求失败: {:#}", e);
                return Err(e.into());
            }
        };
        
        let expected = resp.content_length().unwrap_or_default();
        
        let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
        let received = match tokio::io::copy(&mut stream_reader, &mut file).await {
            Ok(size) => size,
            Err(e) => {
                error!("下载过程中出错: {:#}", e);
                return Err(e.into());
            }
        };
        
        file.flush().await?;
        
        ensure!(
            received >= expected,
            "received {} bytes, expected {} bytes",
            received,
            expected
        );
        
        Ok(())
    }

    /// 多线程分片下载单个文件
    /// 
    /// # 参数
    /// * `url` - 文件下载地址
    /// * `path` - 保存路径
    /// * `concurrency` - 并发数，建议4-8
    pub async fn fetch_parallel(&self, url: &str, path: &Path, concurrency: usize) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        // 使用HEAD请求获取文件总大小
        let resp = match self.client.request(Method::HEAD, url, None).send().await {
            Ok(r) => match r.error_for_status() {
                Ok(r) => r,
                Err(e) => {
                    error!("HTTP状态码错误: {:#}", e);
                    return Err(e.into());
                }
            },
            Err(e) => {
                error!("HTTP请求失败: {:#}", e);
                return Err(e.into());
            }
        };
        
        // 获取文件总大小
        let total_size = resp.headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        
        if total_size == 0 {
            warn!("无法获取文件大小，回退到普通下载方式");
            return self.fetch(url, path).await;
        }
        
        // 检查服务器是否支持Range请求
        if !resp.headers().contains_key("accept-ranges") {
            warn!("服务器不支持Range请求，回退到普通下载方式");
            return self.fetch(url, path).await;
        }
        
        info!("使用多线程下载，文件大小: {} 字节，并发数: {}", total_size, concurrency);
        
        // 创建空文件并预分配空间
        let file = File::create(path).await?;
        file.set_len(total_size).await?;
        drop(file);
        
        // 分块大小
        let chunk_size = total_size / concurrency as u64;
        
        // 创建下载任务
        let mut tasks = Vec::with_capacity(concurrency);
        
        for i in 0..concurrency {
            let start = i as u64 * chunk_size;
            let end = if i == concurrency - 1 {
                total_size - 1
            } else {
                (i + 1) as u64 * chunk_size - 1
            };
            
            let url = url.to_string();
            let path = path.to_path_buf();
            let client = self.client.clone();
            
            // 创建下载任务
            let task = tokio::spawn(async move {
                let mut file = match OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .await {
                    Ok(f) => f,
                    Err(e) => {
                        error!("打开文件失败: {:#}", e);
                        return Err(anyhow::anyhow!("打开文件失败: {:#}", e));
                    }
                };
                
                // 设置Range头
                let range_header = format!("bytes={}-{}", start, end);
                
                // 发送请求
                let resp = match client.request(Method::GET, &url, None)
                    .header(RANGE, &range_header)
                    .send()
                    .await {
                    Ok(r) => match r.error_for_status() {
                        Ok(r) => r,
                        Err(e) => {
                            error!("HTTP状态码错误: {:#}", e);
                            return Err(anyhow::anyhow!("HTTP状态码错误: {:#}", e));
                        }
                    },
                    Err(e) => {
                        error!("HTTP请求失败: {:#}", e);
                        return Err(anyhow::anyhow!("HTTP请求失败: {:#}", e));
                    }
                };
                
                // 将文件指针移动到对应位置
                file.seek(SeekFrom::Start(start)).await?;
                
                // 下载数据并写入文件
                let mut stream = resp.bytes_stream();
                let mut offset = start;
                
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            file.write_all(&chunk).await?;
                            offset += chunk.len() as u64;
                        },
                        Err(e) => {
                            error!("下载分片数据失败: {:#}", e);
                            return Err(anyhow::anyhow!("下载分片数据失败: {:#}", e));
                        }
                    }
                }
                
                file.flush().await?;
                
                // 验证下载的数据大小
                let expected_size = end - start + 1;
                let downloaded_size = offset - start;
                
                ensure!(
                    downloaded_size >= expected_size,
                    "分片 {} 下载不完整: 已下载 {} 字节, 预期 {} 字节",
                    i,
                    downloaded_size,
                    expected_size
                );
                
                Ok::<_, anyhow::Error>(())
            });
            
            tasks.push(task);
        }
        
        // 等待所有任务完成
        let results = future::join_all(tasks).await;
        
        // 检查是否有任务失败
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(Ok(_)) => {},
                Ok(Err(e)) => {
                    error!("分片 {} 下载失败: {:#}", i, e);
                    return Err(e);
                },
                Err(e) => {
                    error!("分片 {} 任务执行失败: {:#}", i, e);
                    return Err(anyhow::anyhow!("分片任务执行失败: {:#}", e));
                }
            }
        }
        
        info!("多线程下载完成: {}", path.display());
        Ok(())
    }

    pub async fn fetch_with_fallback(&self, urls: &[&str], path: &Path) -> Result<()> {
        if urls.is_empty() {
            bail!("no urls provided");
        }
        
        let mut res = Ok(());
        for (_i, url) in urls.iter().enumerate() {
            match self.fetch(url, path).await {
                Ok(_) => {
                    return Ok(());
                },
                Err(err) => {
                    warn!("下载失败: {:#}", err);
                    res = Err(err);
                }
            }
        }
        
        error!("所有URL尝试失败");
        res.with_context(|| format!("failed to download from {:?}", urls))
    }

    /// 使用多线程下载尝试多个URL
    pub async fn fetch_with_fallback_parallel(&self, urls: &[&str], path: &Path, concurrency: usize) -> Result<()> {
        if urls.is_empty() {
            bail!("no urls provided");
        }
        
        let mut res = Ok(());
        for (_i, url) in urls.iter().enumerate() {
            match self.fetch_parallel(url, path, concurrency).await {
                Ok(_) => {
                    return Ok(());
                },
                Err(err) => {
                    warn!("多线程下载失败: {:#}", err);
                    res = Err(err);
                }
            }
        }
        
        error!("所有URL多线程下载尝试失败，回退到普通下载");
        self.fetch_with_fallback(urls, path).await
    }

    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        // 检查输入文件是否存在
        if !video_path.exists() {
            error!("视频文件不存在");
            bail!("视频文件不存在");
        }
        
        if !audio_path.exists() {
            error!("音频文件不存在");
            bail!("音频文件不存在");
        }
        
        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        // 将Path转换为字符串，防止临时值过早释放
        let video_path_str = video_path.to_string_lossy().to_string();
        let audio_path_str = audio_path.to_string_lossy().to_string();
        let output_path_str = output_path.to_string_lossy().to_string();
        
        // 构建FFmpeg命令
        let args = [
            "-i", &video_path_str,
            "-i", &audio_path_str,
            "-c", "copy",
            "-strict", "unofficial",
            "-y",
            &output_path_str,
        ];
        
        let output = tokio::process::Command::new("ffmpeg")
            .args(args)
            .output()
            .await?;
            
        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).unwrap_or("unknown");
            error!("FFmpeg错误: {}", stderr);
            bail!("ffmpeg error: {}", stderr);
        }
        
        Ok(())
    }
}
