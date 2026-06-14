use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BiliError {
    #[error("response missing 'code' field, full response: {0}")]
    InvalidResponse(String),
    #[error("API returned error code {code}, full response: {response}")]
    ErrorResponse {
        code: i64,
        message: Option<String>,
        response: String,
    },
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

    pub fn is_common_error(&self) -> bool {
        if let BiliError::ErrorResponse { code, message, .. } = self {
            for pair in [(-503, "服务暂不可用"), (-504, "服务调用超时")] {
                if *code == pair.0 && message.as_ref().is_some_and(|m| m == pair.1) {
                    return true;
                }
            }
        }
        false
    }
}
