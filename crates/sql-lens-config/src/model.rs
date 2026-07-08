use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
pub struct ProxyConfig {
    pub listen: String,
    pub protocol: Protocol,
    pub capture_mode: CaptureMode,
    pub max_connections: u32,
    pub connect_timeout_ms: u64,
    pub idle_timeout_ms: u64,
    pub shutdown_timeout_ms: u64,
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
            shutdown_timeout_ms: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
pub struct TlsConfig {
    pub mode: TlsMode,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub ca_cert_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
                "passwd".to_owned(),
                "token".to_owned(),
                "secret".to_owned(),
                "api_key".to_owned(),
                "access_key".to_owned(),
                "refresh_token".to_owned(),
            ],
            sql_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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

impl Protocol {
    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::MySql => "mysql",
            Self::PostgreSql => "postgresql",
            Self::Sqlite => "sqlite",
            Self::ClickHouse => "clickhouse",
        }
    }
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
