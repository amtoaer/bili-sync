use std::io;

use anyhow::Result;

pub enum ExecutionStatus {
    Skipped,
    Succeeded,
    Ignored(anyhow::Error),
    Failed(anyhow::Error),
    // 任务可以返回该状态固定自己的 status
    Fixed(u32),
}

// 目前 stable rust 似乎不支持自定义类型使用 ? 运算符，只能先在返回值使用 Result，再这样套层娃
impl From<Result<ExecutionStatus>> for ExecutionStatus {
    fn from(res: Result<ExecutionStatus>) -> Self {
        match res {
            Ok(status) => status,
            Err(err) => {
                for cause in err.chain() {
                    if let Some(io_err) = cause.downcast_ref::<io::Error>() {
                        // 权限错误
                        if io_err.kind() == io::ErrorKind::PermissionDenied {
                            return ExecutionStatus::Ignored(err);
                        }
                        // 使用 io::Error 包裹的 reqwest::Error
                        if io_err.kind() == io::ErrorKind::Other
                            && io_err.get_ref().is_some_and(|e| {
                                e.downcast_ref::<reqwest::Error>().is_some_and(is_ignored_reqwest_error)
                            })
                        {
                            return ExecutionStatus::Ignored(err);
                        }
                    }
                    // 未包裹的 reqwest::Error
                    if let Some(error) = cause.downcast_ref::<reqwest::Error>()
                        && is_ignored_reqwest_error(error)
                    {
                        return ExecutionStatus::Ignored(err);
                    }
                }
                ExecutionStatus::Failed(err)
            }
        }
    }
}

fn is_ignored_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_decode() || err.is_body() || err.is_timeout()
}
