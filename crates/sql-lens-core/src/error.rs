use serde::{Deserialize, Serialize};

use crate::{MetadataField, ProtocolMetadata, RequestId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub code: Option<String>,
    pub sql_state: Option<String>,
    pub message: String,
    pub metadata: Option<ProtocolMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    pub request_id: Option<RequestId>,
    pub details: Vec<MetadataField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    Internal,
    StorageUnavailable,
    ProxyNotReady,
}
