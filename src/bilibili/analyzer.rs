use std::rc::Rc;

use crate::bilibili::Result;

pub struct PageAnalyzer {
    info: serde_json::Value,
}

#[derive(Debug, strum::FromRepr, PartialEq, PartialOrd)]
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
#[derive(Debug, strum::FromRepr, PartialEq, PartialOrd)]
pub enum AudioQuality {
    Quality64k = 30216,
    Quality132k = 30232,
    QualityDolby = 30250,
    QualityHiRES = 30251,
    Quality192k = 30280,
}

#[derive(Debug, strum::EnumString, strum::Display, PartialEq, PartialOrd)]
pub enum VideoCodecs {
    #[strum(serialize = "hev")]
    HEV,
    #[strum(serialize = "avc")]
    AVC,
    #[strum(serialize = "av01")]
    AV1,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Stream {
    FlvStream(String),
    Html5Mp4Stream(String),
    EpositeTryMp4Stream(String),
    DashVideoStream {
        url: String,
        quality: VideoQuality,
        codecs: VideoCodecs,
    },
    DashAudioStream {
        url: String,
        quality: AudioQuality,
    },
}

impl Stream {
    pub fn url(&self) -> &str {
        match self {
            Self::FlvStream(url) => url,
            Self::Html5Mp4Stream(url) => url,
            Self::EpositeTryMp4Stream(url) => url,
            Self::DashVideoStream { url, .. } => url,
            Self::DashAudioStream { url, .. } => url,
        }
    }
}

#[derive(Debug)]
pub enum BestStream {
    VideoAudioStream { video: Stream, audio: Stream },
    MixedStream(Stream),
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

    fn streams(
        &mut self,
        video_max_quality: VideoQuality,
        video_min_quality: VideoQuality,
        audio_max_quality: AudioQuality,
        audio_min_quality: AudioQuality,
        codecs: Rc<Vec<VideoCodecs>>,
        no_dolby_video: bool,
        no_dolby_audio: bool,
        no_hdr: bool,
        no_hires: bool,
    ) -> Result<Vec<Stream>> {
        if self.is_flv_stream() {
            return Ok(vec![Stream::FlvStream(
                self.info["durl"][0]["url"].as_str().unwrap().to_string(),
            )]);
        }
        if self.is_html5_mp4_stream() {
            return Ok(vec![Stream::Html5Mp4Stream(
                self.info["durl"][0]["url"].as_str().unwrap().to_string(),
            )]);
        }
        if self.is_episode_try_mp4_stream() {
            return Ok(vec![Stream::EpositeTryMp4Stream(
                self.info["durl"][0]["url"].as_str().unwrap().to_string(),
            )]);
        }
        let mut streams: Vec<Stream> = Vec::new();
        let videos_data = self.info["dash"]["video"].take();
        let audios_data = self.info["dash"]["audio"].take();
        let flac_data = self.info["dash"]["flac"].take();
        let dolby_data = self.info["dash"]["dolby"].take();
        for video_data in videos_data.as_array().unwrap().iter() {
            let video_stream_url = video_data["baseUrl"].as_str().unwrap().to_string();
            let video_stream_quality =
                VideoQuality::from_repr(video_data["id"].as_u64().unwrap() as usize)
                    .ok_or_else(|| "invalid video stream quality")?;
            if (video_stream_quality == VideoQuality::QualityHdr && no_hdr)  // NO HDR
                || (video_stream_quality == VideoQuality::QualityDolby && no_dolby_video) // NO DOLBY
                || (video_stream_quality != VideoQuality::QualityDolby
                    && video_stream_quality != VideoQuality::QualityHdr
                    && (video_stream_quality < video_min_quality
                        || video_stream_quality > video_max_quality))
            // NOT IN RANGE
            {
                continue;
            }
            let video_codecs = video_data["codecs"].as_str().unwrap();

            let video_codecs = vec![VideoCodecs::HEV, VideoCodecs::AVC, VideoCodecs::AV1]
                .into_iter()
                .filter(|c| video_codecs.contains(c.to_string().as_str()))
                .next();

            let Some(video_codecs) = video_codecs else {
                continue;
            };

            if !codecs.contains(&video_codecs) {
                continue;
            }
            streams.push(Stream::DashVideoStream {
                url: video_stream_url,
                quality: video_stream_quality,
                codecs: video_codecs,
            });
        }
        if audios_data.is_array() {
            for audio_data in audios_data.as_array().unwrap().iter() {
                let audio_stream_url = audio_data["baseUrl"].as_str().unwrap().to_string();
                let audio_stream_quality =
                    AudioQuality::from_repr(audio_data["id"].as_u64().unwrap() as usize);
                let Some(audio_stream_quality) = audio_stream_quality else {
                    continue;
                };
                if audio_stream_quality > audio_max_quality
                    || audio_stream_quality < audio_min_quality
                {
                    continue;
                }
                streams.push(Stream::DashAudioStream {
                    url: audio_stream_url,
                    quality: audio_stream_quality,
                });
            }
        }
        if !(no_hires || flac_data["audio"].is_null()) {
            let flac_stream_url = flac_data["audio"]["baseUrl"].as_str().unwrap().to_string();
            let flac_stream_quality =
                AudioQuality::from_repr(flac_data["audio"]["id"].as_u64().unwrap() as usize)
                    .unwrap();
            streams.push(Stream::DashAudioStream {
                url: flac_stream_url,
                quality: flac_stream_quality,
            });
        }
        if !(no_dolby_audio || dolby_data["audio"].is_null()) {
            let dolby_stream_data = dolby_data["audio"].as_array().and_then(|v| v.get(0));
            if dolby_stream_data.is_some() {
                let dolby_stream_data = dolby_stream_data.unwrap();
                let dolby_stream_url = dolby_stream_data["baseUrl"].as_str().unwrap().to_string();
                let dolby_stream_quality =
                    AudioQuality::from_repr(dolby_stream_data["id"].as_u64().unwrap() as usize)
                        .unwrap();
                streams.push(Stream::DashAudioStream {
                    url: dolby_stream_url,
                    quality: dolby_stream_quality,
                });
            }
        }
        Ok(streams)
    }

    pub fn best_stream(
        &mut self,
        video_max_quality: VideoQuality,
        video_min_quality: VideoQuality,
        audio_max_quality: AudioQuality,
        audio_min_quality: AudioQuality,
        codecs: Vec<VideoCodecs>,
        no_dolby_video: bool,
        no_dolby_audio: bool,
        no_hdr: bool,
        no_hires: bool,
    ) -> Result<BestStream> {
        let codecs = Rc::new(codecs);
        let streams = dbg!(self.streams(
            video_max_quality,
            video_min_quality,
            audio_max_quality,
            audio_min_quality,
            codecs.clone(),
            no_dolby_video,
            no_dolby_audio,
            no_hdr,
            no_hires
        ))?;
        if self.is_flv_stream() || self.is_html5_mp4_stream() || self.is_episode_try_mp4_stream() {
            return Ok(BestStream::MixedStream(
                streams.into_iter().next().ok_or("no stream found")?,
            ));
        }
        let (mut video_streams, mut audio_streams): (Vec<_>, Vec<_>) = streams
            .into_iter()
            .partition(|s| matches!(s, Stream::DashVideoStream { .. }));
        video_streams.sort_by(|a, b| match (a, b) {
            (
                Stream::DashVideoStream {
                    quality: a_quality,
                    codecs: a_codecs,
                    ..
                },
                Stream::DashVideoStream {
                    quality: b_quality,
                    codecs: b_codecs,
                    ..
                },
            ) => {
                if a_quality == &VideoQuality::QualityDolby && !no_dolby_video {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &VideoQuality::QualityDolby && !no_dolby_video {
                    return std::cmp::Ordering::Less;
                }
                if a_quality == &VideoQuality::QualityHdr && !no_hdr {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &VideoQuality::QualityHdr && !no_hdr {
                    return std::cmp::Ordering::Less;
                }
                if a_quality != b_quality {
                    return a_quality.partial_cmp(b_quality).unwrap();
                }
                codecs
                    .iter()
                    .position(|c| c == b_codecs)
                    .cmp(&codecs.iter().position(|c| c == a_codecs))
            }
            _ => std::cmp::Ordering::Equal,
        });
        audio_streams.sort_by(|a, b| match (a, b) {
            (
                Stream::DashAudioStream {
                    quality: a_quality, ..
                },
                Stream::DashAudioStream {
                    quality: b_quality, ..
                },
            ) => {
                if a_quality == &AudioQuality::QualityDolby && !no_dolby_audio {
                    return std::cmp::Ordering::Greater;
                }
                if b_quality == &AudioQuality::QualityDolby && !no_dolby_audio {
                    return std::cmp::Ordering::Less;
                }
                a_quality.partial_cmp(b_quality).unwrap()
            }
            _ => std::cmp::Ordering::Equal,
        });
        if video_streams.is_empty() || audio_streams.is_empty() {
            return Err("no stream found".into());
        }
        Ok(BestStream::VideoAudioStream {
            video: video_streams.remove(video_streams.len() - 1),
            audio: audio_streams.remove(audio_streams.len() - 1),
        })
    }
}
