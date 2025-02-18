use std::io;

use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Request too frequently")]
pub struct DownloadAbortError();

#[derive(Error, Debug)]
#[error("Process page error")]
pub struct ProcessPageError();

pub enum ExecutionResult {
    Skipped,
    Success,
    ErrorIgnored(anyhow::Error),
    Error(anyhow::Error),
}

// 目前 stable rust 似乎不支持自定义类型使用 ? 运算符，只能先在返回值使用 Result，再这样套层娃
impl From<Result<ExecutionResult>> for ExecutionResult {
    fn from(res: Result<ExecutionResult>) -> Self {
        match res {
            Ok(status) => status,
            Err(err) => {
                // error decoding response body
                if let Some(error) = err.downcast_ref::<reqwest::Error>() {
                    if error.is_decode() {
                        return ExecutionResult::ErrorIgnored(err);
                    }
                }
                // 文件系统的权限错误
                if let Some(error) = err.downcast_ref::<io::Error>() {
                    if error.kind() == io::ErrorKind::PermissionDenied {
                        return ExecutionResult::ErrorIgnored(err);
                    }
                }
                ExecutionResult::Error(err)
            }
        }
    }
}
