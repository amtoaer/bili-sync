use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::bilibili::error::BiliError;

pub struct PageAnalyzer {
    pub(crate) info: serde_json::Value,
}

#[derive(Debug, strum::FromRepr, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone)]
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
#[derive(
    Debug, strum::EnumString, strum::Display, strum::AsRefStr, PartialEq, PartialOrd, Serialize, Deserialize, Clone,
)]
pub enum VideoCodecs {
    #[strum(serialize = "hev")]
    HEV,
    #[strum(serialize = "avc")]
    AVC,
    #[strum(serialize = "av01")]
    AV1,
}

impl TryFrom<u64> for VideoCodecs {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        // https://socialsisteryi.github.io/bilibili-API-collect/docs/video/videostream_url.html#%E8%A7%86%E9%A2%91%E7%BC%96%E7%A0%81%E4%BB%A3%E7%A0%81
        match value {
            7 => Ok(Self::AVC),
            12 => Ok(Self::HEV),
            13 => Ok(Self::AV1),
            _ => bail!("invalid video codecs id: {}", value),
        }
    }
}

// 视频流的筛选偏好
#[derive(Serialize, Deserialize, Clone)]
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
        backup_url: Vec<String>,
        quality: VideoQuality,
        codecs: VideoCodecs,
    },
    DashAudio {
        url: String,
        backup_url: Vec<String>,
        quality: AudioQuality,
    },
}

// 通用的获取流链接的方法，交由 Downloader 使用
impl Stream {
    pub fn urls(&self, enable_cdn_sorting: bool) -> Vec<&str> {
        match self {
            Self::Flv(url) | Self::Html5Mp4(url) | Self::EpisodeTryMp4(url) => vec![url],
            Self::DashVideo { url, backup_url, .. } | Self::DashAudio { url, backup_url, .. } => {
                let mut urls = std::iter::once(url.as_str())
                    .chain(backup_url.iter().map(|s| s.as_str()))
                    .collect::<Vec<_>>();
                if enable_cdn_sorting {
                    urls.sort_by_key(|u| {
                        if u.contains("upos-") {
                            0 // 服务商 cdn
                        } else if u.contains("cn-") {
                            1 // 自建 cdn
                        } else if u.contains("mcdn") {
                            2 // mcdn
                        } else {
                            3 // pcdn 或者其它
                        }
                    });
                }
                urls
            }
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
        for video in self
            .info
            .pointer_mut("/dash/video")
            .and_then(|v| v.as_array_mut())
            .ok_or(BiliError::VideoStreamsEmpty)?
            .iter_mut()
        {
            let (Some(url), Some(quality), Some(codecs_id)) = (
                video["baseUrl"].as_str(),
                video["id"].as_u64(),
                video["codecid"].as_u64(),
            ) else {
                continue;
            };
            let quality = VideoQuality::from_repr(quality as usize).context("invalid video stream quality")?;
            let Ok(codecs) = codecs_id.try_into() else {
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
                backup_url: serde_json::from_value(video["backupUrl"].take()).unwrap_or_default(),
                quality,
                codecs,
            });
        }
        if let Some(audios) = self.info.pointer_mut("/dash/audio").and_then(|a| a.as_array_mut()) {
            for audio in audios.iter_mut() {
                let (Some(url), Some(quality)) = (audio["baseUrl"].as_str(), audio["id"].as_u64()) else {
                    continue;
                };
                let quality = AudioQuality::from_repr(quality as usize).context("invalid audio stream quality")?;
                if quality < filter_option.audio_min_quality || quality > filter_option.audio_max_quality {
                    continue;
                }
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    backup_url: serde_json::from_value(audio["backupUrl"].take()).unwrap_or_default(),
                    quality,
                });
            }
        }
        if !filter_option.no_hires
            && let Some(flac) = self.info.pointer_mut("/dash/flac/audio")
        {
            let (Some(url), Some(quality)) = (flac["baseUrl"].as_str(), flac["id"].as_u64()) else {
                bail!("invalid flac stream, flac content: {}", flac);
            };
            let quality = AudioQuality::from_repr(quality as usize).context("invalid flac stream quality")?;
            if quality >= filter_option.audio_min_quality && quality <= filter_option.audio_max_quality {
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    backup_url: serde_json::from_value(flac["backupUrl"].take()).unwrap_or_default(),
                    quality,
                });
            }
        }
        if !filter_option.no_dolby_audio
            && let Some(dolby_audio) = self
                .info
                .pointer_mut("/dash/dolby/audio/0")
                .and_then(|a| a.as_object_mut())
        {
            let (Some(url), Some(quality)) = (dolby_audio["baseUrl"].as_str(), dolby_audio["id"].as_u64()) else {
                bail!("invalid dolby audio stream");
            };
            let quality = AudioQuality::from_repr(quality as usize).context("invalid dolby audio stream quality")?;
            if quality >= filter_option.audio_min_quality && quality <= filter_option.audio_max_quality {
                streams.push(Stream::DashAudio {
                    url: url.to_string(),
                    backup_url: serde_json::from_value(dolby_audio["backupUrl"].take()).unwrap_or_default(),
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
            video: videos
                .into_iter()
                .max_by(|a, b| match (a, b) {
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
            audio: audios.into_iter().max_by(|a, b| match (a, b) {
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
    use crate::config::VersionedConfig;

    #[test]
    fn test_quality_order() {
        assert!(
            [
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
            .is_sorted()
        );
        assert!(
            [
                AudioQuality::Quality64k,
                AudioQuality::Quality132k,
                AudioQuality::Quality192k,
                AudioQuality::QualityDolby,
                AudioQuality::QualityHiRES,
            ]
            .is_sorted()
        );
    }

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_best_stream() {
        let testcases = [
            // 随便一个 8k + hires 视频
            (
                "BV1xRChYUE2R",
                VideoQuality::Quality8k,
                VideoCodecs::HEV,
                Some(AudioQuality::QualityHiRES),
            ),
            // 一个没有声音的纯视频
            ("BV1J7411H7KQ", VideoQuality::Quality720p, VideoCodecs::HEV, None),
            // 一个杜比全景声的演示片
            (
                "BV1Mm4y1P7JV",
                VideoQuality::QualityDolby,
                VideoCodecs::HEV,
                Some(AudioQuality::QualityDolby),
            ),
            // 影视飓风的杜比视界视频
            (
                "BV1HEf2YWEvs",
                VideoQuality::QualityDolby,
                VideoCodecs::HEV,
                Some(AudioQuality::QualityDolby),
            ),
            // 孤独摇滚的杜比视界 + hires + 杜比全景声视频
            (
                "BV1YDVYzeE39",
                VideoQuality::QualityDolby,
                VideoCodecs::HEV,
                Some(AudioQuality::QualityHiRES),
            ),
            // 一个京紫的 HDR 视频
            (
                "BV1cZ4y1b7iB",
                VideoQuality::QualityHdr,
                VideoCodecs::HEV,
                Some(AudioQuality::Quality192k),
            ),
        ];
        let config = VersionedConfig::get().read();
        for (bvid, video_quality, video_codec, audio_quality) in testcases.into_iter() {
            let client = BiliClient::new();
            let video = Video::new(&client, bvid.to_owned(), &config.credential);
            let pages = video.get_pages().await.expect("failed to get pages");
            let first_page = pages.into_iter().next().expect("no page found");
            let best_stream = video
                .get_page_analyzer(&first_page)
                .await
                .expect("failed to get page analyzer")
                .best_stream(&config.filter_option)
                .expect("failed to get best stream");
            dbg!(bvid, &best_stream);
            match best_stream {
                BestStream::VideoAudio {
                    video: Stream::DashVideo { quality, codecs, .. },
                    audio,
                } => {
                    assert_eq!(quality, video_quality);
                    assert_eq!(codecs, video_codec);
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

    #[test]
    fn test_url_sort() {
        let stream = Stream::DashVideo {
            url: "https://xy116x207x155x163xy240ey95dy1010y700yy8dxy.mcdn.bilivideo.cn:4483".to_owned(),
            backup_url: vec![
                "https://upos-sz-mirrorcos.bilivideo.com".to_owned(),
                "https://cn-tj-cu-01-11.bilivideo.com".to_owned(),
                "https://xxx.v1d.szbdys.com".to_owned(),
            ],
            quality: VideoQuality::Quality1080p,
            codecs: VideoCodecs::AVC,
        };
        assert_eq!(
            stream.urls(true),
            vec![
                "https://upos-sz-mirrorcos.bilivideo.com",
                "https://cn-tj-cu-01-11.bilivideo.com",
                "https://xy116x207x155x163xy240ey95dy1010y700yy8dxy.mcdn.bilivideo.cn:4483",
                "https://xxx.v1d.szbdys.com"
            ]
        );
    }
}
