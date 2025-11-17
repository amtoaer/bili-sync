use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BiliError {
    #[error("response missing 'code' or 'message' field, full response: {0}")]
    InvalidResponse(String),
    #[error("API returned error code {0}, message: {1}, full response: {2}")]
    ErrorResponse(i64, String, String),
    #[error("risk control triggered by server, full response: {0}")]
    RiskControlOccurred(String),
    #[error("no video streams available (may indicate risk control)")]
    VideoStreamsEmpty,
}

impl BiliError {
    pub fn is_risk_control_related(&self) -> bool {
        matches!(self, BiliError::RiskControlOccurred(_) | BiliError::VideoStreamsEmpty)
    }
}
