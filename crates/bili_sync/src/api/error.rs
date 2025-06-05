use thiserror::Error;

#[derive(Error, Debug)]
pub enum InnerApiError {
    #[error("Primary key not found: {0}")]
    NotFound(i32),
    #[error("Bad request: {0}")]
    BadRequest(String),
}
