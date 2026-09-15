//! Unified API error type that maps cleanly to HTTP status codes.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// All API errors are represented by this enum.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("entry not found: {0}")]
    EntryNotFound(u64),

    #[error("invalid range: from ({from}) > to ({to})")]
    InvalidRange { from: u64, to: u64 },

    #[error("invalid limit: {0} (must be 1..=100)")]
    InvalidLimit(u32),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("contract not initialized — set CONTRACT_ID env var")]
    ContractNotConfigured,

    #[error("internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

/// JSON body returned for errors.
#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    code: u16,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::EntryNotFound(id) => (
                StatusCode::NOT_FOUND,
                404,
                format!("Entry {id} not found"),
            ),
            ApiError::InvalidRange { from, to } => (
                StatusCode::BAD_REQUEST,
                400,
                format!("Invalid range: from ({from}) must be <= to ({to})"),
            ),
            ApiError::InvalidLimit(l) => (
                StatusCode::BAD_REQUEST,
                400,
                format!("Invalid limit: {l} — must be between 1 and 100"),
            ),
            ApiError::InvalidInput(msg) => (
                StatusCode::BAD_REQUEST,
                400,
                msg.clone(),
            ),
            ApiError::ContractNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                503,
                "Contract not configured — set CONTRACT_ID".into(),
            ),
            ApiError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "Internal server error".into(),
                )
            }
        };

        let label = match status {
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::BAD_REQUEST => "bad_request",
            StatusCode::SERVICE_UNAVAILABLE => "service_unavailable",
            _ => "internal_error",
        };

        (
            status,
            Json(ErrorResponse {
                error: label,
                code,
                message,
            }),
        )
            .into_response()
    }
}
