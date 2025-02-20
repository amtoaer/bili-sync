use anyhow::Error;
use axum::Json;
use axum::response::IntoResponse;
use reqwest::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::error::InnerApiError;

#[derive(ToSchema, Serialize)]
pub struct ApiResponse<T: Serialize> {
    status_code: u16,
    data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { status_code: 200, data }
    }

    pub fn unauthorized(data: T) -> Self {
        Self { status_code: 401, data }
    }

    pub fn not_found(data: T) -> Self {
        Self { status_code: 404, data }
    }

    pub fn internal_server_error(data: T) -> Self {
        Self { status_code: 500, data }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::from_u16(self.status_code).expect("invalid Http Status Code"),
            Json(self),
        )
            .into_response()
    }
}

pub struct ApiError(Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        if let Some(inner_error) = self.0.downcast_ref::<InnerApiError>() {
            match inner_error {
                InnerApiError::NotFound(_) => return ApiResponse::not_found(self.0.to_string()).into_response(),
            }
        }
        ApiResponse::internal_server_error(self.0.to_string()).into_response()
    }
}
