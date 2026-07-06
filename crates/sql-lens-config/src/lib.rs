//! Startup configuration model for SQL Lens.

use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

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

impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let path = path.as_ref();
        let input = fs::read_to_string(path).map_err(|source| ConfigLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_toml_str_with_path(&input, Some(path.to_path_buf()))
    }

    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError> {
        Self::from_toml_str_with_path(input, None)
    }

    fn from_toml_str_with_path(
        input: &str,
        path: Option<PathBuf>,
    ) -> Result<Self, ConfigLoadError> {
        toml::from_str(input).map_err(|source| ConfigLoadError::Parse { path, source })
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        let mut violations = Vec::new();

        if self.proxy.listen.trim().is_empty() {
            violations.push(ConfigValidationViolation::MissingProxyListen);
        }

        if self.backend.address.trim().is_empty() {
            violations.push(ConfigValidationViolation::MissingBackendAddress);
        }

        if self.proxy.protocol != Protocol::MySql {
            violations.push(ConfigValidationViolation::UnsupportedProtocol {
                protocol: self.proxy.protocol,
            });
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(ConfigValidationError { violations })
        }
    }
}

#[derive(Debug)]
pub enum ConfigLoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        source: toml::de::Error,
    },
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "failed to read config file {}: {source}", path.display())
            }
            Self::Parse {
                path: Some(path),
                source,
            } => {
                write!(
                    f,
                    "failed to parse config file {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path: None, source } => {
                write!(f, "failed to parse config: {source}")
            }
        }
    }
}

impl std::error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationError {
    pub violations: Vec<ConfigValidationViolation>,
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.violations.as_slice() {
            [] => write!(f, "invalid config"),
            [violation] => write!(f, "invalid config: {violation}"),
            violations => {
                write!(f, "invalid config: {} violations", violations.len())?;
                for violation in violations {
                    write!(f, "; {violation}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationViolation {
    MissingProxyListen,
    MissingBackendAddress,
    UnsupportedProtocol { protocol: Protocol },
}

impl fmt::Display for ConfigValidationViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProxyListen => write!(f, "`proxy.listen` must not be empty"),
            Self::MissingBackendAddress => write!(f, "`backend.address` must not be empty"),
            Self::UnsupportedProtocol { protocol } => write!(
                f,
                "`proxy.protocol` `{}` is not supported in this build; currently supported: `mysql`",
                protocol.config_value()
            ),
        }
    }
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
                "token".to_owned(),
                "secret".to_owned(),
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
    fn config_value(self) -> &'static str {
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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use std::{
        env, fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    static TEMP_CONFIG_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempConfigFile {
        path: PathBuf,
        dir: PathBuf,
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn assert_serde<T>()
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
    }

    fn assert_error<T>()
    where
        T: std::error::Error,
    {
    }

    fn temp_config_file(contents: &str) -> TempConfigFile {
        let id = TEMP_CONFIG_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("sql-lens-config-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp config dir");

        let path = dir.join("sql-lens.toml");
        fs::write(&path, contents).expect("write temp config file");

        TempConfigFile { path, dir }
    }

    fn missing_config_path() -> PathBuf {
        let id = TEMP_CONFIG_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!(
            "sql-lens-config-missing-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);

        dir.join("sql-lens.toml")
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
        assert_eq!(config.proxy.shutdown_timeout_ms, 10_000);
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

    #[test]
    fn config_load_error_supports_standard_error_traits() {
        assert_error::<ConfigLoadError>();

        let error = SqlLensConfig::from_toml_str("[proxy").expect_err("invalid TOML should fail");

        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn default_config_passes_validation() {
        let config = SqlLensConfig::default();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_validation_error_supports_standard_error_traits() {
        assert_error::<ConfigValidationError>();

        let error = ConfigValidationError {
            violations: vec![ConfigValidationViolation::MissingProxyListen],
        };

        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn missing_proxy_listen_is_rejected() {
        let mut config = SqlLensConfig::default();
        config.proxy.listen.clear();

        let error = config
            .validate()
            .expect_err("missing proxy listen should fail");

        assert_eq!(
            error.violations,
            vec![ConfigValidationViolation::MissingProxyListen]
        );
    }

    #[test]
    fn whitespace_proxy_listen_is_rejected() {
        let mut config = SqlLensConfig::default();
        config.proxy.listen = "   ".to_owned();

        let error = config
            .validate()
            .expect_err("whitespace proxy listen should fail");

        assert_eq!(
            error.violations,
            vec![ConfigValidationViolation::MissingProxyListen]
        );
    }

    #[test]
    fn missing_backend_address_is_rejected() {
        let mut config = SqlLensConfig::default();
        config.backend.address.clear();

        let error = config
            .validate()
            .expect_err("missing backend address should fail");

        assert_eq!(
            error.violations,
            vec![ConfigValidationViolation::MissingBackendAddress]
        );
    }

    #[test]
    fn whitespace_backend_address_is_rejected() {
        let mut config = SqlLensConfig::default();
        config.backend.address = "\t\n".to_owned();

        let error = config
            .validate()
            .expect_err("whitespace backend address should fail");

        assert_eq!(
            error.violations,
            vec![ConfigValidationViolation::MissingBackendAddress]
        );
    }

    #[test]
    fn unsupported_protocol_is_rejected() {
        let mut config = SqlLensConfig::default();
        config.proxy.protocol = Protocol::PostgreSql;

        let error = config
            .validate()
            .expect_err("unsupported protocol should fail");

        assert_eq!(
            error.violations,
            vec![ConfigValidationViolation::UnsupportedProtocol {
                protocol: Protocol::PostgreSql
            }]
        );
    }

    #[test]
    fn validation_collects_multiple_violations() {
        let mut config = SqlLensConfig::default();
        config.proxy.listen = " ".to_owned();
        config.backend.address.clear();
        config.proxy.protocol = Protocol::ClickHouse;

        let error = config
            .validate()
            .expect_err("multiple validation failures should be collected");

        assert_eq!(
            error.violations,
            vec![
                ConfigValidationViolation::MissingProxyListen,
                ConfigValidationViolation::MissingBackendAddress,
                ConfigValidationViolation::UnsupportedProtocol {
                    protocol: Protocol::ClickHouse
                }
            ]
        );
    }

    #[test]
    fn valid_toml_file_loads_from_path() {
        let config_file = temp_config_file(
            r#"
[proxy]
listen = "127.0.0.1:4407"
protocol = "mysql"
capture_mode = "observe"
max_connections = 32
connect_timeout_ms = 1000
idle_timeout_ms = 2000
shutdown_timeout_ms = 3000

[backend]
address = "127.0.0.1:13306"
database_type = "tidb"
tls_server_name = "db.local"

[tls]
mode = "disabled"
client_cert_path = ""
client_key_path = ""
ca_cert_path = ""

[web]
listen = "127.0.0.1:8080"
base_url = "http://127.0.0.1:8080"
cors_origins = ["http://127.0.0.1:8080"]
static_dir = "web/dist"
request_timeout_ms = 5000

[storage]
type = "ring_buffer"
capacity = 42
path = ""

[retention]
max_age = "1h"
max_events = 42
max_bytes = 1048576
drop_policy = "reject_new"

[logging]
level = "debug"
format = "pretty"
redact_secrets = false

[redaction]
enabled = true
mask = "[redacted]"
parameter_names = ["password"]
sql_patterns = ["token"]

[auth]
enabled = true
mode = "local"
session_ttl = "1h"

[replay]
enabled = false
require_confirmation_for_mutations = true

[plugins]
enabled = true
directory = "plugins"
allow_network = false
timeout_ms = 200
"#,
        );

        let config = SqlLensConfig::from_path(&config_file.path).expect("valid config should load");

        assert_eq!(config.proxy.listen, "127.0.0.1:4407");
        assert_eq!(config.proxy.max_connections, 32);
        assert_eq!(config.proxy.shutdown_timeout_ms, 3_000);
        assert_eq!(config.backend.database_type, DatabaseType::TiDb);
        assert_eq!(config.backend.tls_server_name.as_deref(), Some("db.local"));
        assert_eq!(config.tls.mode, TlsMode::Disabled);
        assert_eq!(config.web.static_dir.as_deref(), Some("web/dist"));
        assert_eq!(config.storage.storage_type, StorageType::RingBuffer);
        assert_eq!(config.retention.drop_policy, RetentionDropPolicy::RejectNew);
        assert_eq!(config.logging.level, LoggingLevel::Debug);
        assert_eq!(config.logging.format, LoggingFormat::Pretty);
        assert!(!config.logging.redact_secrets);
        assert_eq!(config.redaction.mask, "[redacted]");
        assert!(config.auth.enabled);
        assert!(!config.replay.enabled);
        assert!(config.plugins.enabled);
        assert_eq!(config.plugins.timeout_ms, 200);
    }

    #[test]
    fn partial_toml_uses_existing_defaults() {
        let config = SqlLensConfig::from_toml_str(
            r#"
[proxy]
listen = "127.0.0.1:4408"

[logging]
level = "debug"
"#,
        )
        .expect("partial config should load");

        assert_eq!(config.proxy.listen, "127.0.0.1:4408");
        assert_eq!(config.proxy.protocol, Protocol::MySql);
        assert_eq!(config.proxy.capture_mode, CaptureMode::Observe);
        assert_eq!(config.proxy.shutdown_timeout_ms, 10_000);
        assert_eq!(config.backend.address, "127.0.0.1:3306");
        assert_eq!(config.storage.capacity, 100_000);
        assert_eq!(config.logging.level, LoggingLevel::Debug);
        assert_eq!(config.logging.format, LoggingFormat::Json);
        assert!(config.redaction.enabled);
        assert_eq!(config.auth.session_ttl, "12h");
    }

    #[test]
    fn invalid_toml_string_returns_parse_error_without_path() {
        let error = SqlLensConfig::from_toml_str("[proxy").expect_err("invalid TOML should fail");

        assert!(matches!(error, ConfigLoadError::Parse { path: None, .. }));
    }

    #[test]
    fn invalid_toml_file_returns_parse_error_with_path() {
        let config_file = temp_config_file("[proxy");

        let error =
            SqlLensConfig::from_path(&config_file.path).expect_err("invalid TOML should fail");

        match error {
            ConfigLoadError::Parse {
                path: Some(path), ..
            } => assert_eq!(path, config_file.path),
            other => panic!("expected parse error with path, got {other:?}"),
        }
    }

    #[test]
    fn missing_config_file_returns_read_error() {
        let path = missing_config_path();

        let error = SqlLensConfig::from_path(&path).expect_err("missing file should fail");

        match error {
            ConfigLoadError::Read {
                path: error_path, ..
            } => assert_eq!(error_path, path),
            other => panic!("expected read error, got {other:?}"),
        }
    }

    #[test]
    fn unknown_toml_sections_and_fields_are_rejected() {
        let unknown_section = SqlLensConfig::from_toml_str(
            r#"
[unknown]
enabled = true
"#,
        )
        .expect_err("unknown sections should fail");

        assert!(matches!(
            unknown_section,
            ConfigLoadError::Parse { path: None, .. }
        ));

        let unknown_field = SqlLensConfig::from_toml_str(
            r#"
[proxy]
lissten = "127.0.0.1:4409"
"#,
        )
        .expect_err("unknown fields should fail");

        assert!(matches!(
            unknown_field,
            ConfigLoadError::Parse { path: None, .. }
        ));
    }
}
