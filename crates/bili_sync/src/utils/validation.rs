use std::path::Path;

use validator::ValidationError;

use crate::utils::status::{STATUS_NOT_STARTED, STATUS_OK};

pub fn validate_status_value(value: u32) -> Result<(), ValidationError> {
    if value == STATUS_OK || value == STATUS_NOT_STARTED {
        Ok(())
    } else {
        Err(ValidationError::new(
            "status_value must be either STATUS_OK or STATUS_NOT_STARTED",
        ))
    }
}

pub fn validate_path(path: &str) -> Result<(), ValidationError> {
    if path.is_empty() || !Path::new(path).is_absolute() {
        Err(ValidationError::new("path must be a non-empty absolute path"))
    } else {
        Ok(())
    }
}
