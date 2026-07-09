//! Startup configuration model for SQL Lens.

mod error;
mod loading;
mod model;
#[cfg(test)]
mod tests;
mod validation;

pub use error::{ConfigLoadError, ConfigValidationError, ConfigValidationViolation};
pub use model::{
    AuthConfig, AuthMode, BackendConfig, CaptureMode, DatabaseType, LoggingConfig, LoggingFormat,
    LoggingLevel, PluginsConfig, Protocol, ProxyConfig, ProxyTargetConfig, RedactionConfig,
    ReplayConfig, RetentionConfig, RetentionDropPolicy, SqlLensConfig, StorageConfig, StorageType,
    TlsConfig, TlsMode, WebConfig,
};
