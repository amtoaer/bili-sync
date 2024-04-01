use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use crate::bilibili::error::BiliError;

pub struct PageAnalyzer {
    info: serde_json::Value,
}

#[derive(Debug, strum::FromRepr, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum VideoQuality {
    Quality360p = 16,
    Quality480p = 32,
    Quality720p = 64,
    Quality1080p = 80,
    Quality1080pPLUS = 112,
    Quality1080p60 = 116,
    Quality4k = 120,
    QualityHdr = 125,
    QualityDolby = 126,
    Quality8k = 127,
}
#[derive(Debug, strum::FromRepr, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum AudioQuality {
    Quality64k = 30216,
    Quality132k = 30232,
    QualityDolby = 30250,
    QualityHiRES = 30251,
    Quality192k = 30280,
}

#[derive(Debug, strum::EnumString, strum::Display, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum VideoCodecs {
    #[strum(serialize = "hev")]
    HEV,
    #[strum(serialize = "avc")]
    AVC,
    #[strum(serialize = "av01")]
    AV1,
}

// 视频流的筛选偏好
#[derive(Serialize, Deserialize)]
pub struct FilterOption {
    pub video_max_quality: VideoQuality,
    pub video_min_quality: VideoQuality,
    pub audio_max_quality: AudioQuality,
    pub audio_min_quality: AudioQuality,
    pub codecs: Arc<Vec<VideoCodecs>>,
    pub no_dolby_video: bool,
    pub no_dolby_audio: bool,
    pub no_hdr: bool,
    pub no_hires: bool,
}

impl Default for FilterOption {
    fn default() -> Self {
        Self {
            video_max_quality: VideoQuality::Quality8k,
            video_min_quality: VideoQuality::Quality360p,
            audio_max_quality: AudioQuality::QualityHiRES,
            audio_min_quality: AudioQuality::Quality64k,
            codecs: Arc::new(vec![VideoCodecs::AV1, VideoCodecs::HEV, VideoCodecs::AVC]),
            no_dolby_video: false,
            no_dolby_audio: false,
            no_hdr: false,
            no_hires: false,
        }
    }
}

// 上游项目中的五种流类型，不过目测应该只有 Flv、DashVideo、DashAudio 三种会被用到
#[derive(Debug, PartialEq, PartialOrd)]
pub enum Stream {
    Flv(String),
    Html5Mp4(String),
    EpositeTryMp4(String),
    DashVideo {
        url: String,
        quality: VideoQuality,
        codecs: VideoCodecs,
    },
    DashAudio {
        url: String,
        quality: AudioQuality,
    },
}

// 通用的获取流链接的方法，交由 Downloader 使用
impl Stream {
    pub fn url(&self) -> &str {
        match self {
            Self::Flv(url) => url,
            Self::Html5Mp4(url) => url,
            Self::EpositeTryMp4(url) => url,
            Self::DashVideo { url, .. } => url,
            Self::DashAudio { url, .. } => url,
        }
    }
}

/// 用于获取视频流的最佳筛选结果，有两种可能：
/// 1. 单个混合流，作为 Mixed 返回
/// 2. 视频、音频分离，作为 VideoAudio 返回，其中音频流可能不存在（对于无声视频，如 BV1J7411H7KQ）
#[derive(Debug)]
pub enum BestStream {
    VideoAudio { video: Stream, audio: Option<Stream> },
    Mixed(Stream),
}

impl PageAnalyzer {
    pub fn new(info: serde_json::Value) -> Self {
        Self { info }
    }

    fn is_flv_stream(&self) -> bool {
        self.info.get("durl").is_some()
            && self.info["format"].is_string()
            && self.info["format"].as_str().unwrap().starts_with("flv")
    }

    fn is_html5_mp4_stream(&self) -> bool {
        self.info.get("durl").is_some()
            && self.info["format"].is_string()
            && self.info["format"].as_str().unwrap().starts_with("mp4")
            && self.info["is_html5"].is_boolean()
            && self.info["is_html5"].as_bool().unwrap()
    }

    fn is_episode_try_mp4_stream(&self) -> bool {
        self.info.get("durl").is_some()
            && self.info["format"].is_string()
            && self.info["format"].as_str().unwrap().starts_with("mp4")
            && !(self.info["is_html5"].is_boolean() && self.info["is_html5"].as_bool().unwrap())
    }

    fn streams(&mut self, filter_option: &FilterOption) -> Result<Vec<Stream>> {
        if self.is_flv_stream() {
            return Ok(vec![Stream::Flv(
                self.info["durl"][0]["url"]
                    .as_str()
                    .ok_or(anyhow!("invalid flv stream"))?
                    .to_string(),
            )]);
        }
        if self.is_html5_mp4_stream() {
            return Ok(vec![Stream::Html5Mp4(
                self.info["durl"][0]["url"]
                    .as_str()
                    .ok_or(anyhow!("invalid html5 mp4 stream"))?
                    .to_string(),
            )]);
        }
        if self.is_episode_try_mp4_stream() {
            return Ok(vec![Stream::EpositeTryMp4(
                self.info["durl"][0]["url"]
                    .as_str()
                    .ok_or(anyhow!("invalid episode try mp4 stream"))?
                    .to_string(),
            )]);
        }
        let mut streams: Vec<Stream> = Vec::new();
        let videos_data = self.info["dash"]["video"].take();
        let audios_data = self.info["dash"]["audio"].take();
        let flac_data = self.info["dash"]["flac"].take();
        let dolby_data = self.info["dash"]["dolby"].take();
        for video_data in videos_data.as_array().ok_or(BiliError::RiskControlOccurred)?.iter() {
            let video_stream_url = video_data["baseUrl"].as_str().unwrap().to_string();
            let video_stream_quality = VideoQuality::from_repr(video_data["id"].as_u64().unwrap() as usize)
                .ok_or(anyhow!("invalid video stream quality"))?;
            if (video_stream_quality == VideoQuality::QualityHdr && filter_option.no_hdr)
                || (video_stream_quality == VideoQuality::QualityDolby && filter_option.no_dolby_video)
                || (video_stream_quality != VideoQuality::QualityDolby
                    && video_stream_quality != VideoQuality::QualityHdr
                    && (video_stream_quality < filter_option.video_min_quality
                        || video_stream_quality > filter_option.video_max_quality))
            // 此处过滤包含三种情况：
            // 1. HDR 视频，但指定不需要 HDR
            // 2. 杜比视界视频，但指定不需要杜比视界
            // 3. 视频质量不在指定范围内
            {
                continue;
            }

            let video_codecs = video_data["codecs"].as_str().unwrap();
            // 从视频流的 codecs 字段中获取编码格式，此处并非精确匹配而是判断包含，比如 codecs 是 av1.42c01e，需要匹配为 av1
            let video_codecs = vec![VideoCodecs::HEV, VideoCodecs::AVC, VideoCodecs::AV1]
                .into_iter()
                .find(|c| video_codecs.contains(c.to_string().as_str()));

            let Some(video_codecs) = video_codecs else {
                continue;
            };
            if !filter_option.codecs.contains(&video_codecs) {
                continue;
            }
            streams.push(Stream::DashVideo {
                url: video_stream_url,
                quality: video_stream_quality,
                codecs: video_codecs,
            });
        }
        if audios_data.is_array() {
            for audio_data in audios_data.as_array().unwrap().iter() {
                let audio_stream_url = audio_data["baseUrl"].as_str().unwrap().to_string();
                let audio_stream_quality = AudioQuality::from_repr(audio_data["id"].as_u64().unwrap() as usize);
                let Some(audio_stream_quality) = audio_stream_quality else {
                    continue;
                };
                if audio_stream_quality > filter_option.audio_max_quality
                    || audio_stream_quality < filter_option.audio_min_quality
                {
                    continue;
                }
                streams.push(Stream::DashAudio {
                    url: audio_stream_url,
                    quality: audio_stream_quality,
                });
            }
        }
        if !(filter_option.no_hires || flac_data["audio"].is_null()) {
            // 允许 hires 且存在 flac 音频流才会进来
            let flac_stream_url = flac_data["audio"]["baseUrl"].as_str().unwrap().to_string();
            let flac_stream_quality =
                AudioQuality::from_repr(flac_data["audio"]["id"].as_u64().unwrap() as usize).unwrap();
            streams.push(Stream::DashAudio {
                url: flac_stream_url,
                quality: flac_stream_quality,
            });
        }
        if !(filter_option.no_dolby_audio || dolby_data["audio"].is_null()) {
            // 同理，允许杜比音频且存在杜比音频流才会进来
            let dolby_stream_data = dolby_data["audio"].as_array().and_then(|v| v.first());
            if dolby_stream_data.is_some() {
                let dolby_stream_data = dolby_stream_data.unwrap();
                let dolby_stream_url = dolby_stream_data["baseUrl"].as_str().unwrap().to_string();
                let dolby_stream_quality =
                    AudioQuality::from_repr(dolby_stream_data["id"].as_u64().unwrap() as usize).unwrap();
                streams.push(Stream::DashAudio {
                    url: dolby_stream_url,
                    quality: dolby_stream_quality,
                });
            }
        }
        Ok(streams)
    }

    pub fn best_stream(&mut self, filter_option: &FilterOption) -> Result<BestStream> {
        let streams = self.streams(filter_option)?;
        if self.is_flv_stream() || self.is_html5_mp4_stream() || self.is_episode_try_mp4_stream() {
            // 按照 streams 中的假设，符合这三种情况的流只有一个，直接取
            return Ok(BestStream::Mixed(streams.into_iter().next().unwrap()));
        }
        // 将视频流和音频流拆分，分别做排序
        let (mut video_streams, mut audio_streams): (Vec<_>, Vec<_>) =
            streams.into_iter().partition(|s| matches!(s, Stream::DashVideo { .. }));
        // 因为该处的排序与筛选选项有关，因此不能在外面实现 PartialOrd trait，只能在这里写闭包
        video_streams.sort_by(|a, b| match (a, b) {
            (
                Stream::DashVideo {
                    quality: a_quality,
                    codecs: a_codecs,
                    ..
                },
                Stream::DashVideo {
                    quality: b_quality,
                    codecs: b_codecs,
                    ..
                },
            ) => {
                if a_quality == &VideoQuality::QualityDolby && !filter_option.no_dolby_video {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &VideoQuality::QualityDolby && !filter_option.no_dolby_video {
                    return std::cmp::Ordering::Less;
                }
                if a_quality == &VideoQuality::QualityHdr && !filter_option.no_hdr {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &VideoQuality::QualityHdr && !filter_option.no_hdr {
                    return std::cmp::Ordering::Less;
                }
                if a_quality != b_quality {
                    return a_quality.partial_cmp(b_quality).unwrap();
                }
                // 如果视频质量相同，按照偏好的编码优先级排序
                filter_option
                    .codecs
                    .iter()
                    .position(|c| c == b_codecs)
                    .cmp(&filter_option.codecs.iter().position(|c| c == a_codecs))
            }
            _ => unreachable!(),
        });
        audio_streams.sort_by(|a, b| match (a, b) {
            (Stream::DashAudio { quality: a_quality, .. }, Stream::DashAudio { quality: b_quality, .. }) => {
                if a_quality == &AudioQuality::QualityDolby && !filter_option.no_dolby_audio {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &AudioQuality::QualityDolby && !filter_option.no_dolby_audio {
                    return std::cmp::Ordering::Less;
                }
                a_quality.partial_cmp(b_quality).unwrap()
            }
            _ => unreachable!(),
        });
        if video_streams.is_empty() {
            bail!("no video stream found");
        }
        Ok(BestStream::VideoAudio {
            video: video_streams.remove(video_streams.len() - 1),
            // 音频流可能为空，因此直接使用 pop 返回 Option
            audio: audio_streams.pop(),
        })
    }
}
