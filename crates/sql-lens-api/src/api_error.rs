use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sql_lens_core::ApiErrorCode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiErrorBody {
    code: String,
    message: String,
    request_id: Option<String>,
    details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiEndpointError {
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
    details: BTreeMap<String, String>,
}

impl ApiEndpointError {
    pub(crate) fn bad_request(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: ApiErrorCode::BadRequest,
            message: message.into(),
            details: BTreeMap::from([("field".to_owned(), field.into())]),
        }
    }

    pub(crate) fn not_found(
        message: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: ApiErrorCode::NotFound,
            message: message.into(),
            details: BTreeMap::from([(key.into(), value.into())]),
        }
    }
}

impl IntoResponse for ApiEndpointError {
    fn into_response(self) -> Response {
        let body = ApiErrorEnvelope {
            error: ApiErrorBody {
                code: api_error_code_name(self.code).to_owned(),
                message: self.message,
                request_id: None,
                details: self.details,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

fn api_error_code_name(code: ApiErrorCode) -> &'static str {
    match code {
        ApiErrorCode::BadRequest => "BAD_REQUEST",
        ApiErrorCode::Unauthorized => "UNAUTHORIZED",
        ApiErrorCode::Forbidden => "FORBIDDEN",
        ApiErrorCode::NotFound => "NOT_FOUND",
        ApiErrorCode::Conflict => "CONFLICT",
        ApiErrorCode::RateLimited => "RATE_LIMITED",
        ApiErrorCode::Internal => "INTERNAL",
        ApiErrorCode::StorageUnavailable => "STORAGE_UNAVAILABLE",
        ApiErrorCode::ProxyNotReady => "PROXY_NOT_READY",
    }
}
