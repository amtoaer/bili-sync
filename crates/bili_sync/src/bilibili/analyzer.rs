use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::bilibili::error::BiliError;

pub struct PageAnalyzer {
    info: serde_json::Value,
}

#[derive(Debug, strum::FromRepr, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, strum::FromRepr, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioQuality {
    Quality64k = 30216,
    Quality132k = 30232,
    QualityDolby = 30250,
    QualityHiRES = 30251,
    Quality192k = 30280,
}

impl Ord for AudioQuality {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_sort_key().cmp(&other.as_sort_key())
    }
}

impl PartialOrd for AudioQuality {
    fn partial_cmp(&self, other: &AudioQuality) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AudioQuality {
    pub fn as_sort_key(&self) -> isize {
        match self {
            // 这可以让 Dolby 和 Hi-RES 排在 192k 之后，且 Dolby 和 Hi-RES 之间的顺序不变
            Self::QualityHiRES | Self::QualityDolby => (*self as isize) + 40,
            _ => *self as isize,
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, strum::EnumString, strum::Display, strum::AsRefStr, PartialEq, PartialOrd, Serialize, Deserialize)]
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
    pub codecs: Vec<VideoCodecs>,
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
            codecs: vec![VideoCodecs::AV1, VideoCodecs::HEV, VideoCodecs::AVC],
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
    EpisodeTryMp4(String),
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
            Self::EpisodeTryMp4(url) => url,
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
        self.info.get("durl").is_some() && self.info["format"].as_str().is_some_and(|f| f.starts_with("flv"))
    }

    fn is_html5_mp4_stream(&self) -> bool {
        self.info.get("durl").is_some()
            && self.info["format"].as_str().is_some_and(|f| f.starts_with("mp4"))
            && self.info["is_html5"].as_bool().is_some_and(|b| b)
    }

    fn is_episode_try_mp4_stream(&self) -> bool {
        self.info.get("durl").is_some()
            && self.info["format"].as_str().is_some_and(|f| f.starts_with("mp4"))
            && self.info["is_html5"].as_bool().is_none_or(|b| !b)
    }

    /// 获取所有的视频、音频流，并根据条件筛选
    fn streams(&mut self, filter_option: &FilterOption) -> Result<Vec<Stream>> {
        if self.is_flv_stream() {
            return Ok(vec![Stream::Flv(
                self.info["durl"][0]["url"]
                    .as_str()
                    .context("invalid flv stream")?
                    .to_string(),
            )]);
        }
        if self.is_html5_mp4_stream() {
            return Ok(vec![Stream::Html5Mp4(
                self.info["durl"][0]["url"]
                    .as_str()
                    .context("invalid html5 mp4 stream")?
                    .to_string(),
            )]);
        }
        if self.is_episode_try_mp4_stream() {
            return Ok(vec![Stream::EpisodeTryMp4(
                self.info["durl"][0]["url"]
                    .as_str()
                    .context("invalid episode try mp4 stream")?
                    .to_string(),
            )]);
        }
        let mut streams: Vec<Stream> = Vec::new();
        for video in self.info["dash"]["video"]
            .as_array()
            .ok_or(BiliError::RiskControlOccurred)?
            .iter()
        {
            let (Some(url), Some(quality), Some(codecs)) = (
                video["baseUrl"].as_str(),
                video["id"].as_u64(),
                video["codecs"].as_str(),
            ) else {
                continue;
            };
            let quality = VideoQuality::from_repr(quality as usize).context("invalid video stream quality")?;
            // 从视频流的 codecs 字段中获取编码格式，此处并非精确匹配而是判断包含，比如 codecs 是 av1.42c01e，需要匹配为 av1
            let Some(codecs) = [VideoCodecs::HEV, VideoCodecs::AVC, VideoCodecs::AV1]
                .into_iter()
                .find(|c| codecs.contains(c.as_ref()))
            else {
                // 少数情况会走到此处，如 codecs 为 dvh1.08.09、hvc1.2.4.L123.90 等，直接跳过，不影响流程
                continue;
            };
            if !filter_option.codecs.contains(&codecs)
                || quality < filter_option.video_min_quality
                || quality > filter_option.video_max_quality
                || (quality == VideoQuality::QualityHdr && filter_option.no_hdr)
                || (quality == VideoQuality::QualityDolby && filter_option.no_dolby_video)
            {
                continue;
            }
            streams.push(Stream::DashVideo {
                url: url.to_string(),
                quality,
                codecs,
            });
        }
        if let Some(audios) = self.info["dash"]["audio"].as_array() {
            for audio in audios.iter() {
                let (Some(url), Some(quality)) = (audio["baseUrl"].as_str(), audio["id"].as_u64()) else {
                    continue;
                };
                let quality = AudioQuality::from_repr(quality as usize).context("invalid audio stream quality")?;
                if quality < filter_option.audio_min_quality || quality > filter_option.audio_max_quality {
                    continue;
                }
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    quality,
                });
            }
        }
        let flac = &self.info["dash"]["flac"]["audio"];
        if !(filter_option.no_hires || flac.is_null()) {
            let (Some(url), Some(quality)) = (flac["baseUrl"].as_str(), flac["id"].as_u64()) else {
                bail!("invalid flac stream");
            };
            let quality = AudioQuality::from_repr(quality as usize).context("invalid flac stream quality")?;
            if quality >= filter_option.audio_min_quality && quality <= filter_option.audio_max_quality {
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    quality,
                });
            }
        }
        let dolby_audio = &self.info["dash"]["dolby"]["audio"][0];
        if !(filter_option.no_dolby_audio || dolby_audio.is_null()) {
            let (Some(url), Some(quality)) = (dolby_audio["baseUrl"].as_str(), dolby_audio["id"].as_u64()) else {
                bail!("invalid dolby audio stream");
            };
            let quality = AudioQuality::from_repr(quality as usize).context("invalid dolby audio stream quality")?;
            if quality >= filter_option.audio_min_quality && quality <= filter_option.audio_max_quality {
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    quality,
                });
            }
        }
        Ok(streams)
    }

    pub fn best_stream(&mut self, filter_option: &FilterOption) -> Result<BestStream> {
        let streams = self.streams(filter_option)?;
        if self.is_flv_stream() || self.is_html5_mp4_stream() || self.is_episode_try_mp4_stream() {
            // 按照 streams 中的假设，符合这三种情况的流只有一个，直接取
            return Ok(BestStream::Mixed(
                streams.into_iter().next().context("no stream found")?,
            ));
        }
        let (videos, audios): (Vec<Stream>, Vec<Stream>) =
            streams.into_iter().partition(|s| matches!(s, Stream::DashVideo { .. }));
        Ok(BestStream::VideoAudio {
            video: Iterator::max_by(videos.into_iter(), |a, b| match (a, b) {
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
                    if a_quality != b_quality {
                        return a_quality.cmp(b_quality);
                    };
                    filter_option
                        .codecs
                        .iter()
                        .position(|c| c == b_codecs)
                        .cmp(&filter_option.codecs.iter().position(|c| c == a_codecs))
                }
                _ => unreachable!(),
            })
            .context("no video stream found")?,
            audio: Iterator::max_by(audios.into_iter(), |a, b| match (a, b) {
                (Stream::DashAudio { quality: a_quality, .. }, Stream::DashAudio { quality: b_quality, .. }) => {
                    a_quality.cmp(b_quality)
                }
                _ => unreachable!(),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::{BiliClient, Video};
    use crate::config::CONFIG;

    #[test]
    fn test_quality_order() {
        assert!([
            VideoQuality::Quality360p,
            VideoQuality::Quality480p,
            VideoQuality::Quality720p,
            VideoQuality::Quality1080p,
            VideoQuality::Quality1080pPLUS,
            VideoQuality::Quality1080p60,
            VideoQuality::Quality4k,
            VideoQuality::QualityHdr,
            VideoQuality::QualityDolby,
            VideoQuality::Quality8k
        ]
        .is_sorted());
        assert!([
            AudioQuality::Quality64k,
            AudioQuality::Quality132k,
            AudioQuality::Quality192k,
            AudioQuality::QualityDolby,
            AudioQuality::QualityHiRES,
        ]
        .is_sorted());
    }

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_best_stream() {
        let testcases = [
            // 随便一个 8k + hires 视频
            (
                "BV1xRChYUE2R",
                VideoQuality::Quality8k,
                Some(AudioQuality::QualityHiRES),
            ),
            // 一个没有声音的纯视频
            ("BV1J7411H7KQ", VideoQuality::Quality720p, None),
            // 一个杜比全景声的演示片
            (
                "BV1Mm4y1P7JV",
                VideoQuality::Quality4k,
                Some(AudioQuality::QualityDolby),
            ),
        ];
        for (bvid, video_quality, audio_quality) in testcases.into_iter() {
            let client = BiliClient::new();
            let video = Video::new(&client, bvid.to_owned());
            let pages = video.get_pages().await.expect("failed to get pages");
            let first_page = pages.into_iter().next().expect("no page found");
            let best_stream = video
                .get_page_analyzer(&first_page)
                .await
                .expect("failed to get page analyzer")
                .best_stream(&CONFIG.filter_option)
                .expect("failed to get best stream");
            dbg!(bvid, &best_stream);
            match best_stream {
                BestStream::VideoAudio {
                    video: Stream::DashVideo { quality, .. },
                    audio,
                } => {
                    assert_eq!(quality, video_quality);
                    assert_eq!(
                        audio.map(|audio_stream| match audio_stream {
                            Stream::DashAudio { quality, .. } => quality,
                            _ => unreachable!(),
                        }),
                        audio_quality,
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
