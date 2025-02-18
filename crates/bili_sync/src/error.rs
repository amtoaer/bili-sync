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
}

// 目前 stable rust 似乎不支持自定义类型使用 ? 运算符，只能先在返回值使用 Result，再这样套层娃
impl From<Result<ExecutionStatus>> for ExecutionStatus {
    fn from(res: Result<ExecutionStatus>) -> Self {
        match res {
            Ok(status) => status,
            Err(err) => {
                // error decoding response body
                if let Some(error) = err.downcast_ref::<reqwest::Error>() {
                    if error.is_decode() {
                        return ExecutionStatus::Ignored(err);
                    }
                }
                // 文件系统的权限错误
                if let Some(error) = err.downcast_ref::<io::Error>() {
                    if error.kind() == io::ErrorKind::PermissionDenied {
                        return ExecutionStatus::Ignored(err);
                    }
                }
                ExecutionStatus::Failed(err)
            }
        }
    }
}
