use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use tokio::process::Command;

use crate::config::{CONFIG_DIR, YoutubeSkipOption, YoutubeVideoFormat};
use crate::utils::{compact_log_filename, compact_log_path, compact_log_text};

const BRIDGE_PY: &str = include_str!("python/bridge.py");
const CORE_PY: &str = include_str!("python/core.py");

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Subscription {
    pub channel_id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Playlist {
    pub playlist_id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub owner_name: Option<String>,
    #[serde(default)]
    pub video_count: Option<usize>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedSourceKind {
    Channel,
    Playlist,
    Video,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedSource {
    pub kind: ResolvedSourceKind,
    pub source_id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub owner_name: Option<String>,
    #[serde(default)]
    pub video_count: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SourceVideo {
    pub video_id: String,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    pub uploader: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DownloadResult {
    pub output_dir: String,
    pub video_file: String,
}

pub async fn list_subscriptions(cookie_path: &Path) -> Result<Vec<Subscription>> {
    run_bridge_json::<Vec<Subscription>, _>([
        "subscriptions".to_owned(),
        "--cookie-file".to_owned(),
        cookie_path.to_string_lossy().into_owned(),
    ])
    .await
}

pub async fn list_playlists(cookie_path: &Path) -> Result<Vec<Playlist>> {
    run_bridge_json::<Vec<Playlist>, _>([
        "playlists".to_owned(),
        "--cookie-file".to_owned(),
        cookie_path.to_string_lossy().into_owned(),
    ])
    .await
}

pub async fn resolve_source(url: &str, cookie_path: Option<&Path>) -> Result<ResolvedSource> {
    let mut args = vec!["resolve-source".to_owned(), "--url".to_owned(), url.to_owned()];
    if let Some(cookie_path) = cookie_path {
        args.push("--cookie-file".to_owned());
        args.push(cookie_path.to_string_lossy().into_owned());
    }
    run_bridge_json(args).await
}

pub async fn list_playlist_videos(url: &str, cookie_path: Option<&Path>) -> Result<Vec<SourceVideo>> {
    let mut args = vec!["playlist-videos".to_owned(), "--url".to_owned(), url.to_owned()];
    if let Some(cookie_path) = cookie_path {
        args.push("--cookie-file".to_owned());
        args.push(cookie_path.to_string_lossy().into_owned());
    }
    run_bridge_json(args).await
}

pub async fn download_video(
    url: &str,
    output_dir: &Path,
    cookie_path: Option<&Path>,
    skip_option: &YoutubeSkipOption,
    video_format: YoutubeVideoFormat,
) -> Result<DownloadResult> {
    let mut args = vec![
        "download".to_owned(),
        "--url".to_owned(),
        url.to_owned(),
        "--output-dir".to_owned(),
        output_dir.to_string_lossy().into_owned(),
        "--output-format".to_owned(),
        video_format.as_str().to_owned(),
    ];
    if let Some(cookie_path) = cookie_path {
        args.push("--cookie-file".to_owned());
        args.push(cookie_path.to_string_lossy().into_owned());
    }
    if skip_option.no_poster {
        args.push("--skip-poster".to_owned());
    }
    if skip_option.no_video_nfo {
        args.push("--skip-nfo".to_owned());
    }
    if skip_option.no_subtitle {
        args.push("--skip-subtitle".to_owned());
    }
    run_bridge_json(args).await
}

async fn run_bridge_json<T, I>(args: I) -> Result<T>
where
    T: DeserializeOwned,
    I: IntoIterator<Item = String>,
{
    let python = detect_python().await?;
    let bridge_path = ensure_helper_scripts().await?;

    let output = Command::new(&python)
        .arg(&bridge_path)
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to run youtube helper with {}", python))?;

    for line in String::from_utf8_lossy(&output.stderr).lines() {
        if !line.trim().is_empty()
            && let Some(line) = compact_youtube_helper_log(line)
        {
            info!("[YouTube Helper] {}", line);
        }
    }

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "youtube helper failed: status={} stdout={} stderr={}",
            output.status,
            stdout.trim(),
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("youtube helper stdout is not valid utf-8")?;
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("failed to parse youtube helper output: {}", stdout.trim()))
}

async fn ensure_helper_scripts() -> Result<PathBuf> {
    let helper_dir = CONFIG_DIR.join("youtube_helper");
    tokio::fs::create_dir_all(&helper_dir)
        .await
        .with_context(|| format!("failed to create youtube helper dir {}", helper_dir.display()))?;

    let bridge_path = helper_dir.join("bridge.py");
    let core_path = helper_dir.join("core.py");

    tokio::fs::write(&bridge_path, BRIDGE_PY)
        .await
        .with_context(|| format!("failed to write youtube helper {}", bridge_path.display()))?;
    tokio::fs::write(&core_path, CORE_PY)
        .await
        .with_context(|| format!("failed to write youtube helper {}", core_path.display()))?;

    Ok(bridge_path)
}

async fn detect_python() -> Result<String> {
    for candidate in ["python3", "python"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
        {
            return Ok(candidate.to_owned());
        }
    }
    bail!("python3/python not found, YouTube helper is unavailable")
}

pub fn compact_download_file_log(file: &str) -> String {
    compact_log_filename(file, 48)
}

fn compact_youtube_helper_log(line: &str) -> Option<String> {
    let line = line.trim();

    if let Some((prefix, path)) = line.split_once("Destination: ") {
        return Some(format!("{prefix}Destination: {}", compact_log_path(path, 48)));
    }
    if let Some((prefix, path)) = line.split_once("Merging formats into ") {
        return Some(format!("{prefix}Merging formats into {}", compact_log_path(path, 48)));
    }
    if let Some((prefix, rest)) = line.split_once("Deleting original file ") {
        let (path, suffix) = rest
            .split_once(" (")
            .map(|(path, suffix)| (path, format!(" ({suffix}")))
            .unwrap_or((rest, String::new()));
        return Some(format!(
            "{prefix}Deleting original file {}{}",
            compact_log_path(path, 48),
            suffix
        ));
    }
    if let Some((prefix, _)) = line.split_once(" has already been downloaded") {
        if let Some((head, path)) = prefix.rsplit_once(' ') {
            return Some(format!(
                "{head} {} has already been downloaded",
                compact_log_path(path, 48)
            ));
        }
        return Some(format!("{} has already been downloaded", compact_log_path(prefix, 48)));
    }
    if line.starts_with("[download]")
        || line.starts_with("WARNING: [youtube]")
        || line.starts_with("WARNING: [generic]")
    {
        return None;
    }

    Some(compact_log_text(line, 120))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_helper_destination_log_uses_short_filename() {
        let line = "[download] Destination: /media/youtube/download/some/very/very/long/file-name-example.mp4";
        let compacted = compact_youtube_helper_log(line);
        assert!(
            compacted
                .as_ref()
                .unwrap()
                .contains("Destination: file-name-example.mp4")
        );
        assert!(!compacted.as_ref().unwrap().contains("/media/youtube/download"));
    }

    #[test]
    fn compact_helper_filters_progress_lines() {
        let line = "[download]  37.4% of  123.45MiB at    4.32MiB/s ETA 00:18";
        assert!(compact_youtube_helper_log(line).is_none());
    }

    #[test]
    fn compact_download_file_log_preserves_extension() {
        let long_name = format!("{}示例文件.mp4", "这是非常长的文件名".repeat(10));
        let compacted = compact_download_file_log(&long_name);
        assert!(compacted.ends_with(".mp4"));
        assert!(compacted.contains('…'));
    }
}
