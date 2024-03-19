use std::path::Path;
use std::rc::Rc;

use bili_sync::bilibili::{
    AudioQuality, BestStream, BiliClient, FavoriteList, FilterOption, Video, VideoCodecs,
    VideoQuality,
};
use bili_sync::downloader::Downloader;
use futures_util::{pin_mut, StreamExt};

#[tokio::main]
async fn main() {
    let bili_client = Rc::new(BiliClient::new(None));
    let favorite_list = FavoriteList::new(bili_client.clone(), "52642258".to_string());
    dbg!(favorite_list.get_info().await.unwrap());

    let video_stream = favorite_list.into_video_stream();
    // from doc: https://docs.rs/async-stream/latest/async_stream/
    pin_mut!(video_stream);

    let third_video_info = dbg!(video_stream.skip(2).next().await.unwrap());
    let third_video = Video::new(bili_client.clone(), third_video_info.bvid);
    dbg!(third_video.get_tags().await.unwrap());

    let pages = dbg!(third_video.get_pages().await.unwrap());
    let best_stream = dbg!(third_video
        .get_page_analyzer(&pages[0])
        .await
        .unwrap()
        .best_stream(&FilterOption {
            video_max_quality: VideoQuality::QualityDolby,
            video_min_quality: VideoQuality::Quality360p,
            audio_max_quality: AudioQuality::QualityDolby,
            audio_min_quality: AudioQuality::Quality64k,
            codecs: Rc::new(vec![VideoCodecs::HEV, VideoCodecs::AVC]),
            no_dolby_video: false,
            no_dolby_audio: false,
            no_hdr: false,
            no_hires: false,
        }))
    .unwrap();

    let downloader = Downloader::default();
    let base = Path::new("./");
    let output_path = base.join(format!("{}.mp4", third_video_info.title));

    match best_stream {
        BestStream::Mixed(stream) => {
            let url = dbg!(stream.url());
            downloader.fetch(url, &output_path).await.unwrap();
        }
        BestStream::VideoAudio { video, audio } => {
            let url = dbg!(video.url());
            let Some(audio) = audio else {
                downloader.fetch(url, &output_path).await.unwrap();
                return;
            };
            let video_path = base.join(format!("{}_video_tmp", third_video_info.title));
            downloader.fetch(url, &video_path).await.unwrap();
            let url = dbg!(audio.url());
            let audio_path = base.join(format!("{}_audio_tmp", third_video_info.title));
            downloader.fetch(url, &audio_path).await.unwrap();
            downloader
                .merge(&video_path, &audio_path, &output_path)
                .await
                .unwrap();
        }
    }
}
