use thiserror::Error;

#[derive(Error, Debug)]
pub enum BiliError {
    #[error("risk control occurred")]
    RiskControlOccurred,
    #[error("request failed, status code: {0}, message: {1}")]
    RequestFailed(u64, String),
}
