use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing_subscriber::fmt::MakeWriter;

pub const MAX_HISTORY_LOGS: usize = 200;

/// LogHelper 维护了日志发送器和一个日志历史记录的缓冲区
pub struct LogHelper {
    pub sender: broadcast::Sender<String>,
    pub log_history: Arc<RwLock<VecDeque<String>>>,
}

impl LogHelper {
    pub fn new(sender: broadcast::Sender<String>, log_history: Arc<RwLock<VecDeque<String>>>) -> Self {
        LogHelper { sender, log_history }
    }
}

impl<'a> MakeWriter<'a> for LogHelper {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl std::io::Write for LogHelper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let log_message = String::from_utf8_lossy(buf).to_string();
        let _ = self.sender.send(log_message.clone());
        let mut history = self.log_history.write();
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

impl Clone for LogHelper {
    fn clone(&self) -> Self {
        LogHelper {
            sender: self.sender.clone(),
            log_history: self.log_history.clone(),
        }
    }
}
