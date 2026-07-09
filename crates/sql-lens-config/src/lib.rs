//! Startup configuration model for SQL Lens.

mod env_overrides;
mod error;
mod loading;
mod model;
#[cfg(test)]
mod tests;
mod validation;

pub use env_overrides::{SQL_LENS_BACKEND_ADDRESS, SQL_LENS_LOGGING_LEVEL, SQL_LENS_PROXY_LISTEN};
pub use error::{
    ConfigLoadError, ConfigOverrideError, ConfigValidationError, ConfigValidationViolation,
};
pub use model::{
    BackendConfig, CaptureMode, DatabaseType, LoggingConfig, LoggingFormat, LoggingLevel,
    PluginsConfig, Protocol, ProxyConfig, ProxyTargetConfig, RedactionConfig, ReplayConfig,
    RetentionConfig, RetentionDropPolicy, SqlLensConfig, StorageConfig, StorageType, TlsConfig,
    TlsMode, WebConfig,
};
