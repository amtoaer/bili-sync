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
