use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sql_lens_core::ApiErrorCode;
use utoipa::ToSchema;

use crate::RequestId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiEndpointError {
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
    details: BTreeMap<String, String>,
}

impl ApiEndpointError {
    fn new(
        code: ApiErrorCode,
        message: impl Into<String>,
        details: BTreeMap<String, String>,
    ) -> Self {
        Self {
            status: api_error_status(code),
            code,
            message: message.into(),
            details,
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::new(
            ApiErrorCode::BadRequest,
            message,
            BTreeMap::from([("field".to_owned(), field.into())]),
        )
    }

    pub(crate) fn not_found(
        message: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self::new(
            ApiErrorCode::NotFound,
            message,
            BTreeMap::from([(key.into(), value.into())]),
        )
    }

    pub(crate) fn storage_unavailable(message: impl Into<String>) -> Self {
        Self::new(ApiErrorCode::StorageUnavailable, message, BTreeMap::new())
    }
}

impl IntoResponse for ApiEndpointError {
    fn into_response(self) -> Response {
        let parts = ApiErrorResponseParts {
            body: ApiErrorBody {
                code: api_error_code_name(self.code).to_owned(),
                message: self.message,
                request_id: None,
                details: self.details,
            },
        };

        let mut response = (self.status, Json(parts.envelope())).into_response();
        response.extensions_mut().insert(parts);
        response
    }
}

#[derive(Debug, Clone)]
struct ApiErrorResponseParts {
    body: ApiErrorBody,
}

impl ApiErrorResponseParts {
    fn envelope(&self) -> ApiErrorEnvelope {
        ApiErrorEnvelope {
            error: self.body.clone(),
        }
    }

    fn envelope_with_request_id(&self, request_id: &RequestId) -> ApiErrorEnvelope {
        let mut body = self.body.clone();
        body.request_id = Some(request_id.as_str().to_owned());

        ApiErrorEnvelope { error: body }
    }
}

pub(crate) fn with_request_id(mut response: Response, request_id: &RequestId) -> Response {
    let Some(parts) = response.extensions_mut().remove::<ApiErrorResponseParts>() else {
        return response;
    };

    let status = response.status();
    (status, Json(parts.envelope_with_request_id(request_id))).into_response()
}

fn api_error_status(code: ApiErrorCode) -> StatusCode {
    match code {
        ApiErrorCode::BadRequest => StatusCode::BAD_REQUEST,
        ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
        ApiErrorCode::Conflict => StatusCode::CONFLICT,
        ApiErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        ApiErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::StorageUnavailable | ApiErrorCode::ProxyNotReady => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

fn api_error_code_name(code: ApiErrorCode) -> &'static str {
    match code {
        ApiErrorCode::BadRequest => "BAD_REQUEST",
        ApiErrorCode::NotFound => "NOT_FOUND",
        ApiErrorCode::Conflict => "CONFLICT",
        ApiErrorCode::RateLimited => "RATE_LIMITED",
        ApiErrorCode::Internal => "INTERNAL",
        ApiErrorCode::StorageUnavailable => "STORAGE_UNAVAILABLE",
        ApiErrorCode::ProxyNotReady => "PROXY_NOT_READY",
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use sql_lens_core::ApiErrorCode;

    #[test]
    fn api_error_codes_map_to_documented_status_and_code_names() {
        let cases = [
            (
                ApiErrorCode::BadRequest,
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
            ),
            (ApiErrorCode::NotFound, StatusCode::NOT_FOUND, "NOT_FOUND"),
            (ApiErrorCode::Conflict, StatusCode::CONFLICT, "CONFLICT"),
            (
                ApiErrorCode::RateLimited,
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
            ),
            (
                ApiErrorCode::Internal,
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
            ),
            (
                ApiErrorCode::StorageUnavailable,
                StatusCode::SERVICE_UNAVAILABLE,
                "STORAGE_UNAVAILABLE",
            ),
            (
                ApiErrorCode::ProxyNotReady,
                StatusCode::SERVICE_UNAVAILABLE,
                "PROXY_NOT_READY",
            ),
        ];

        for (code, expected_status, expected_code) in cases {
            assert_eq!(super::api_error_status(code), expected_status);
            assert_eq!(super::api_error_code_name(code), expected_code);
        }
    }
}
