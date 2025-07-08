use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::broadcast;
use tracing_subscriber::fmt::MakeWriter;

const MAX_HISTORY_LOGS: usize = 20;

pub struct MpscWriter {
    pub sender: broadcast::Sender<String>,
    pub log_history: Arc<Mutex<VecDeque<String>>>,
}

impl MpscWriter {
    pub fn new(sender: broadcast::Sender<String>, log_history: Arc<Mutex<VecDeque<String>>>) -> Self {
        MpscWriter { sender, log_history }
    }
}

impl<'a> MakeWriter<'a> for MpscWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl std::io::Write for MpscWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let log_message = String::from_utf8_lossy(buf).to_string();
        let _ = self.sender.send(log_message.clone());
        let mut history = self.log_history.lock();
        history.push_back(log_message);
        if history.len() > MAX_HISTORY_LOGS {
            history.pop_front();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Clone for MpscWriter {
    fn clone(&self) -> Self {
        MpscWriter {
            sender: self.sender.clone(),
            log_history: self.log_history.clone(),
        }
    }
}
