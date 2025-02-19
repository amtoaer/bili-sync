use std::io;

use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Request too frequently")]
pub struct DownloadAbortError();

#[derive(Error, Debug)]
#[error("Process page error")]
pub struct ProcessPageError();

pub enum ExecutionStatus {
    Skipped,
    Succeeded,
    Ignored(anyhow::Error),
    Failed(anyhow::Error),
    // 任务可以返回该状态固定自己的 status
    FixedFailed(u32, anyhow::Error),
}

// 目前 stable rust 似乎不支持自定义类型使用 ? 运算符，只能先在返回值使用 Result，再这样套层娃
impl From<Result<ExecutionStatus>> for ExecutionStatus {
    fn from(res: Result<ExecutionStatus>) -> Self {
        match res {
            Ok(status) => status,
            Err(err) => {
                if let Some(error) = err.downcast_ref::<io::Error>() {
                    let error_kind = error.kind();
                    if error_kind == io::ErrorKind::PermissionDenied
                        || (error_kind == io::ErrorKind::Other
                            && error.get_ref().is_some_and(|e| {
                                e.downcast_ref::<reqwest::Error>()
                                    .is_some_and(|e| e.is_decode() || e.is_body() || e.is_timeout())
                            }))
                    {
                        return ExecutionStatus::Ignored(err);
                    }
                }
                if let Some(error) = err.downcast_ref::<reqwest::Error>() {
                    if error.is_decode() || error.is_body() || error.is_timeout() {
                        return ExecutionStatus::Ignored(err);
                    }
                }
                ExecutionStatus::Failed(err)
            }
        }
    }
}
