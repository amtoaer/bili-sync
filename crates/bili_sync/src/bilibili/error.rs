use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BiliError {
    #[error("response missing 'code' or 'message' field, full response: {0}")]
    InvalidResponse(String),
    #[error("API returned error code {0}, full response: {1}")]
    ErrorResponse(i64, String),
    #[error("risk control triggered by server, full response: {0}")]
    RiskControlOccurred(String),
    #[error("invalid HTTP response code {0}, reason: {1}")]
    InvalidStatusCode(u16, &'static str),
    #[error("no video streams available (may indicate risk control)")]
    VideoStreamsEmpty,
}

impl BiliError {
    pub fn is_risk_control_related(&self) -> bool {
        matches!(
            self,
            BiliError::RiskControlOccurred(_) | BiliError::VideoStreamsEmpty | BiliError::InvalidStatusCode(_, _)
        )
    }
}
