use std::rc::Rc;

use bili_sync::bilibili::{
    AudioQuality, BiliClient, FavoriteList, Video, VideoCodecs, VideoQuality,
};
use futures_util::{pin_mut, StreamExt};

#[tokio::main]
async fn main() {
    let bili_client = Rc::new(BiliClient::anonymous());
    let favorite_list = FavoriteList::new(bili_client.clone(), "52642258".to_string());
    dbg!(favorite_list.get_info().await.unwrap());
    let video_stream = favorite_list.into_video_stream();
    // from doc: https://docs.rs/async-stream/latest/async_stream/
    pin_mut!(video_stream);
    let mut count = 3;
    let mut third_video = None;
    while let Some(mut video) = video_stream.next().await {
        count -= 1;
        video = dbg!(video);
        if count <= 0 {
            third_video = Some(video);
            break;
        }
    }
    let third_video = Video::new(bili_client.clone(), third_video.unwrap().bvid);
    dbg!(third_video.get_tags().await.unwrap());
    let pages = dbg!(third_video.get_pages().await.unwrap());
    dbg!(third_video
        .get_page_analyzer(&pages[0])
        .await
        .unwrap()
        .best_stream(
            VideoQuality::QualityDolby,
            VideoQuality::Quality360p,
            AudioQuality::QualityDolby,
            AudioQuality::Quality64k,
            vec![VideoCodecs::HEV, VideoCodecs::AVC],
            false,
            false,
            false,
            false,
        ))
    .unwrap();
}
