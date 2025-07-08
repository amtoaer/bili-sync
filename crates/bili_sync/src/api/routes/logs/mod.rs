mod mpsc;
use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{Extension, Router};
use futures::{Stream, StreamExt};
pub use mpsc::MpscWriter;
use tokio_stream::wrappers::BroadcastStream;

pub(super) fn router() -> Router {
    Router::new().route("/logs", get(logs))
}

async fn logs(Extension(log_writer): Extension<MpscWriter>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let history = log_writer.log_history.lock();
    let rx = log_writer.sender.subscribe();
    let history_logs: Vec<String> = history.iter().cloned().collect();
    drop(history);

    let history_stream = { futures::stream::iter(history_logs.into_iter().map(|msg| Ok(Event::default().data(msg)))) };

    let stream = BroadcastStream::new(rx).filter_map(async |msg| match msg {
        Ok(log_message) => Some(Ok(Event::default().data(log_message))),
        Err(e) => {
            error!("Broadcast stream error: {:?}", e);
            None
        }
    });
    Sse::new(history_stream.chain(stream)).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}
