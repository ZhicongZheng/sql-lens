//! Startup configuration model for SQL Lens.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SqlLensConfig {
    pub proxy: ProxyConfig,
    pub backend: BackendConfig,
    pub tls: TlsConfig,
    pub web: WebConfig,
    pub storage: StorageConfig,
    pub retention: RetentionConfig,
    pub logging: LoggingConfig,
    pub redaction: RedactionConfig,
    pub auth: AuthConfig,
    pub replay: ReplayConfig,
    pub plugins: PluginsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub listen: String,
    pub protocol: Protocol,
    pub capture_mode: CaptureMode,
    pub max_connections: u32,
    pub connect_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:3307".to_owned(),
            protocol: Protocol::default(),
            capture_mode: CaptureMode::default(),
            max_connections: 512,
            connect_timeout_ms: 5_000,
            idle_timeout_ms: 300_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendConfig {
    pub address: String,
    pub database_type: DatabaseType,
    pub tls_server_name: Option<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:3306".to_owned(),
            database_type: DatabaseType::default(),
            tls_server_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    pub mode: TlsMode,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub ca_cert_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebConfig {
    pub listen: String,
    pub base_url: String,
    pub cors_origins: Vec<String>,
    pub static_dir: Option<String>,
    pub request_timeout_ms: u64,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:5173".to_owned(),
            base_url: "http://127.0.0.1:5173".to_owned(),
            cors_origins: vec!["http://127.0.0.1:5173".to_owned()],
            static_dir: None,
            request_timeout_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(rename = "type")]
    pub storage_type: StorageType,
    pub capacity: u64,
    pub path: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::default(),
            capacity: 100_000,
            path: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionConfig {
    pub max_age: String,
    pub max_events: u64,
    pub max_bytes: Option<u64>,
    pub drop_policy: RetentionDropPolicy,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            max_age: "24h".to_owned(),
            max_events: 100_000,
            max_bytes: None,
            drop_policy: RetentionDropPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LoggingLevel,
    pub format: LoggingFormat,
    pub redact_secrets: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LoggingLevel::default(),
            format: LoggingFormat::default(),
            redact_secrets: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionConfig {
    pub enabled: bool,
    pub mask: String,
    pub parameter_names: Vec<String>,
    pub sql_patterns: Vec<String>,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mask: "***".to_owned(),
            parameter_names: vec![
                "password".to_owned(),
                "token".to_owned(),
                "secret".to_owned(),
            ],
            sql_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub mode: AuthMode,
    pub session_ttl: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AuthMode::default(),
            session_ttl: "12h".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub enabled: bool,
    pub require_confirmation_for_mutations: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_confirmation_for_mutations: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginsConfig {
    pub enabled: bool,
    pub directory: String,
    pub allow_network: bool,
    pub timeout_ms: u64,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory: "plugins".to_owned(),
            allow_network: false,
            timeout_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Protocol {
    #[default]
    #[serde(rename = "mysql")]
    MySql,
    #[serde(rename = "postgresql")]
    PostgreSql,
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "clickhouse")]
    ClickHouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DatabaseType {
    #[default]
    #[serde(rename = "mysql")]
    MySql,
    #[serde(rename = "starrocks")]
    StarRocks,
    #[serde(rename = "tidb")]
    TiDb,
    #[serde(rename = "doris")]
    Doris,
    #[serde(rename = "postgresql")]
    PostgreSql,
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "clickhouse")]
    ClickHouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    Disabled,
    #[default]
    Passthrough,
    Terminate,
    Upstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StorageType {
    #[default]
    #[serde(rename = "ring_buffer")]
    RingBuffer,
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "duckdb")]
    DuckDb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoggingLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoggingFormat {
    #[default]
    Json,
    Pretty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Disabled,
    #[default]
    Local,
    Oidc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetentionDropPolicy {
    #[default]
    Oldest,
    RejectNew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    #[default]
    Observe,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    fn assert_serde<T>()
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
    }

    #[test]
    fn default_config_contains_documented_proxy_and_backend_defaults() {
        let config = SqlLensConfig::default();

        assert_eq!(config.proxy.listen, "127.0.0.1:3307");
        assert_eq!(config.proxy.protocol, Protocol::MySql);
        assert_eq!(config.proxy.capture_mode, CaptureMode::Observe);
        assert_eq!(config.proxy.max_connections, 512);
        assert_eq!(config.proxy.connect_timeout_ms, 5_000);
        assert_eq!(config.proxy.idle_timeout_ms, 300_000);
        assert_eq!(config.backend.address, "127.0.0.1:3306");
        assert_eq!(config.backend.database_type, DatabaseType::MySql);
        assert_eq!(config.tls.mode, TlsMode::Passthrough);
        assert_eq!(config.tls.client_cert_path, "");
        assert_eq!(config.tls.client_key_path, "");
        assert_eq!(config.tls.ca_cert_path, "");
    }

    #[test]
    fn default_config_contains_documented_web_storage_and_retention_defaults() {
        let config = SqlLensConfig::default();

        assert_eq!(config.web.listen, "127.0.0.1:5173");
        assert_eq!(config.web.base_url, "http://127.0.0.1:5173");
        assert_eq!(
            config.web.cors_origins,
            vec!["http://127.0.0.1:5173".to_owned()]
        );
        assert_eq!(config.storage.storage_type, StorageType::RingBuffer);
        assert_eq!(config.storage.capacity, 100_000);
        assert_eq!(config.storage.path, "");
        assert_eq!(config.retention.max_age, "24h");
        assert_eq!(config.retention.max_events, 100_000);
        assert_eq!(config.retention.drop_policy, RetentionDropPolicy::Oldest);
    }

    #[test]
    fn default_config_contains_documented_security_and_extension_defaults() {
        let config = SqlLensConfig::default();

        assert_eq!(config.logging.level, LoggingLevel::Info);
        assert_eq!(config.logging.format, LoggingFormat::Json);
        assert!(config.logging.redact_secrets);
        assert!(config.redaction.enabled);
        assert_eq!(config.redaction.mask, "***");
        assert_eq!(
            config.redaction.parameter_names,
            vec![
                "password".to_owned(),
                "token".to_owned(),
                "secret".to_owned()
            ]
        );
        assert!(!config.auth.enabled);
        assert_eq!(config.auth.mode, AuthMode::Local);
        assert_eq!(config.auth.session_ttl, "12h");
        assert!(config.replay.enabled);
        assert!(config.replay.require_confirmation_for_mutations);
        assert!(!config.plugins.enabled);
        assert_eq!(config.plugins.directory, "plugins");
    }

    #[test]
    fn public_config_types_support_serde_traits() {
        assert_serde::<SqlLensConfig>();
        assert_serde::<ProxyConfig>();
        assert_serde::<BackendConfig>();
        assert_serde::<StorageConfig>();
        assert_serde::<Protocol>();
        assert_serde::<DatabaseType>();
        assert_serde::<StorageType>();
        assert_serde::<CaptureMode>();
    }
}
