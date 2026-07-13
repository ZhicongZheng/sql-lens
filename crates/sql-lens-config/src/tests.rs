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
    assert_eq!(config.proxy.slow_threshold_ms, 500);
    assert_eq!(config.proxy.max_connections, 512);
    assert_eq!(config.proxy.connect_timeout_ms, 5_000);
    assert_eq!(config.proxy.idle_timeout_ms, 300_000);
    assert_eq!(config.proxy.shutdown_timeout_ms, 10_000);
    assert_eq!(config.capture.capacity, 1_024);
    assert_eq!(
        config.capture.overload_policy,
        CaptureOverloadPolicy::DropNewest
    );
    assert!(config.targets.is_empty());
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
    assert!(config.retention.enforcement_enabled);
    assert_eq!(config.retention.enforcement_interval, "1h");
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
            "passwd".to_owned(),
            "token".to_owned(),
            "secret".to_owned(),
            "api_key".to_owned(),
            "access_key".to_owned(),
            "refresh_token".to_owned()
        ]
    );
    assert!(config.replay.enabled);
    assert!(config.replay.require_confirmation_for_mutations);
    assert!(!config.plugins.enabled);
    assert_eq!(config.plugins.directory, "plugins");
}

#[test]
fn public_config_types_support_serde_traits() {
    assert_serde::<SqlLensConfig>();
    assert_serde::<ProxyConfig>();
    assert_serde::<ProxyTargetConfig>();
    assert_serde::<BackendConfig>();
    assert_serde::<StorageConfig>();
    assert_serde::<Protocol>();
    assert_serde::<DatabaseType>();
    assert_serde::<StorageType>();
    assert_serde::<CaptureMode>();
    assert_serde::<CaptureConfig>();
    assert_serde::<CaptureOverloadPolicy>();
}

#[test]
fn config_load_error_supports_standard_error_traits() {
    assert_error::<ConfigLoadError>();

    let error = SqlLensConfig::from_toml_str("[proxy").expect_err("invalid TOML should fail");

    assert!(!error.to_string().is_empty());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn config_override_error_supports_standard_error_traits() {
    assert_error::<ConfigOverrideError>();

    let mut config = SqlLensConfig::default();
    let error = config
        .apply_env_overrides_from([(SQL_LENS_LOGGING_LEVEL, "verbose")])
        .expect_err("invalid logging override should fail");

    assert_eq!(error.variable, SQL_LENS_LOGGING_LEVEL);
    assert_eq!(error.value, "verbose");
    assert!(!error.to_string().is_empty());
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn default_config_passes_validation() {
    let config = SqlLensConfig::default();

    assert!(config.validate().is_ok());
}

#[test]
fn capture_config_parses_both_overload_policies_and_rejects_zero_capacity() {
    let config = SqlLensConfig::from_toml_str(
        r#"
[capture]
capacity = 42
overload_policy = "reject_new"
"#,
    )
    .expect("capture configuration should parse");

    assert_eq!(config.capture.capacity, 42);
    assert_eq!(
        config.capture.overload_policy,
        CaptureOverloadPolicy::RejectNew
    );
    assert!(config.validate().is_ok());

    let zero_capacity = SqlLensConfig::from_toml_str(
        r#"
[capture]
capacity = 0
overload_policy = "drop_newest"
"#,
    )
    .expect("zero capacity configuration should parse before validation");
    let error = zero_capacity
        .validate()
        .expect_err("zero capture capacity should fail validation");

    assert_eq!(
        error.violations,
        vec![ConfigValidationViolation::InvalidCaptureCapacity]
    );
}

#[test]
fn default_config_expands_to_legacy_effective_target() {
    let config = SqlLensConfig::default();

    assert_eq!(
        config.effective_targets(),
        vec![ProxyTargetConfig {
            name: "default".to_owned(),
            listen: "127.0.0.1:3307".to_owned(),
            protocol: Protocol::MySql,
            database_type: DatabaseType::MySql,
            backend_address: "127.0.0.1:3306".to_owned(),
        }]
    );
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
fn multi_target_toml_loads_and_expands_configured_targets() {
    let config = SqlLensConfig::from_toml_str(
        r#"
[[targets]]
name = "mysql-local"
listen = "127.0.0.1:3307"
protocol = "mysql"
database_type = "mysql"
backend_address = "127.0.0.1:3306"

[[targets]]
name = "starrocks-local"
listen = "127.0.0.1:9037"
protocol = "mysql"
database_type = "starrocks"
backend_address = "127.0.0.1:9030"
"#,
    )
    .expect("multi-target config should load");

    assert!(config.validate().is_ok());
    assert_eq!(
        config.effective_targets(),
        vec![
            ProxyTargetConfig {
                name: "mysql-local".to_owned(),
                listen: "127.0.0.1:3307".to_owned(),
                protocol: Protocol::MySql,
                database_type: DatabaseType::MySql,
                backend_address: "127.0.0.1:3306".to_owned(),
            },
            ProxyTargetConfig {
                name: "starrocks-local".to_owned(),
                listen: "127.0.0.1:9037".to_owned(),
                protocol: Protocol::MySql,
                database_type: DatabaseType::StarRocks,
                backend_address: "127.0.0.1:9030".to_owned(),
            },
        ]
    );
}

#[test]
fn multi_target_validation_rejects_missing_values() {
    let config = SqlLensConfig::from_toml_str(
        r#"
[[targets]]
name = ""
listen = ""
protocol = "mysql"
database_type = "mysql"
backend_address = ""
"#,
    )
    .expect("semantically invalid target config should still parse");

    let error = config
        .validate()
        .expect_err("missing target fields should fail validation");

    assert_eq!(
        error.violations,
        vec![
            ConfigValidationViolation::MissingTargetName { index: 0 },
            ConfigValidationViolation::MissingTargetListen {
                target_name: "#0".to_owned(),
            },
            ConfigValidationViolation::MissingTargetBackendAddress {
                target_name: "#0".to_owned(),
            },
        ]
    );
}

#[test]
fn multi_target_validation_rejects_duplicates_and_unsupported_protocol() {
    let config = SqlLensConfig::from_toml_str(
        r#"
[[targets]]
name = "analytics"
listen = "127.0.0.1:3307"
protocol = "postgresql"
database_type = "postgresql"
backend_address = "127.0.0.1:5432"

[[targets]]
name = "analytics"
listen = "127.0.0.1:3307"
protocol = "mysql"
database_type = "mysql"
backend_address = "127.0.0.1:3306"
"#,
    )
    .expect("duplicate target config should still parse");

    let error = config
        .validate()
        .expect_err("duplicate and unsupported target values should fail validation");

    assert_eq!(
        error.violations,
        vec![
            ConfigValidationViolation::UnsupportedTargetProtocol {
                target_name: "analytics".to_owned(),
                protocol: Protocol::PostgreSql,
            },
            ConfigValidationViolation::DuplicateTargetName {
                name: "analytics".to_owned(),
            },
            ConfigValidationViolation::DuplicateTargetListen {
                listen: "127.0.0.1:3307".to_owned(),
            },
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
slow_threshold_ms = 250
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
enforcement_enabled = false
enforcement_interval = "15m"

[logging]
level = "debug"
format = "pretty"
redact_secrets = false

[redaction]
enabled = true
mask = "[redacted]"
parameter_names = ["password"]
sql_patterns = ["token"]

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
    assert_eq!(config.proxy.slow_threshold_ms, 250);
    assert_eq!(config.proxy.max_connections, 32);
    assert_eq!(config.proxy.shutdown_timeout_ms, 3_000);
    assert_eq!(config.backend.database_type, DatabaseType::TiDb);
    assert_eq!(config.backend.tls_server_name.as_deref(), Some("db.local"));
    assert_eq!(config.tls.mode, TlsMode::Disabled);
    assert_eq!(config.web.static_dir.as_deref(), Some("web/dist"));
    assert_eq!(config.storage.storage_type, StorageType::RingBuffer);
    assert_eq!(config.retention.drop_policy, RetentionDropPolicy::RejectNew);
    assert!(!config.retention.enforcement_enabled);
    assert_eq!(config.retention.enforcement_interval, "15m");
    assert_eq!(config.logging.level, LoggingLevel::Debug);
    assert_eq!(config.logging.format, LoggingFormat::Pretty);
    assert!(!config.logging.redact_secrets);
    assert_eq!(config.redaction.mask, "[redacted]");
    assert!(!config.replay.enabled);
    assert!(config.plugins.enabled);
    assert_eq!(config.plugins.timeout_ms, 200);
}

#[test]
fn retention_enforcement_interval_requires_a_positive_supported_duration() {
    assert_eq!(
        parse_retention_enforcement_interval("10ms"),
        Some(std::time::Duration::from_millis(10))
    );
    assert_eq!(
        parse_retention_enforcement_interval("1h"),
        Some(std::time::Duration::from_secs(3_600))
    );

    let mut config = SqlLensConfig::default();
    config.retention.enforcement_interval = "0".to_owned();

    let error = config
        .validate()
        .expect_err("zero retention enforcement interval should fail validation");
    assert_eq!(
        error.violations,
        vec![ConfigValidationViolation::InvalidRetentionEnforcementInterval]
    );
}

#[test]
fn retention_validation_rejects_invalid_age_and_unsupported_bytes() {
    let mut config = SqlLensConfig::default();
    config.retention.max_age = "not-a-duration".to_owned();
    config.retention.max_bytes = Some(1_024);

    let error = config
        .validate()
        .expect_err("unsupported retention settings should fail validation");

    assert_eq!(
        error.violations,
        vec![
            ConfigValidationViolation::InvalidRetentionMaxAge,
            ConfigValidationViolation::UnsupportedRetentionMaxBytes,
        ]
    );
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
    assert_eq!(config.proxy.slow_threshold_ms, 500);
    assert_eq!(config.proxy.shutdown_timeout_ms, 10_000);
    assert_eq!(config.backend.address, "127.0.0.1:3306");
    assert_eq!(config.storage.capacity, 100_000);
    assert_eq!(config.logging.level, LoggingLevel::Debug);
    assert_eq!(config.logging.format, LoggingFormat::Json);
    assert!(config.redaction.enabled);
}

#[test]
fn environment_overrides_proxy_backend_and_logging_level() {
    let mut config = SqlLensConfig::from_toml_str(
        r#"
[proxy]
listen = "127.0.0.1:4408"

[backend]
address = "127.0.0.1:3306"

[logging]
level = "info"
"#,
    )
    .expect("base config should load");

    config
        .apply_env_overrides_from([
            (SQL_LENS_PROXY_LISTEN, "127.0.0.1:5507"),
            (SQL_LENS_BACKEND_ADDRESS, "127.0.0.1:23306"),
            (SQL_LENS_LOGGING_LEVEL, "debug"),
        ])
        .expect("valid overrides should apply");

    assert_eq!(config.proxy.listen, "127.0.0.1:5507");
    assert_eq!(config.backend.address, "127.0.0.1:23306");
    assert_eq!(config.logging.level, LoggingLevel::Debug);
}

#[test]
fn unknown_environment_overrides_are_ignored() {
    let mut config = SqlLensConfig::default();

    config
        .apply_env_overrides_from([("SQL_LENS_UNKNOWN", "value")])
        .expect("unknown overrides should be ignored");

    assert_eq!(config, SqlLensConfig::default());
}

#[test]
fn environment_overrides_do_not_rewrite_explicit_targets() {
    let mut config = SqlLensConfig::from_toml_str(
        r#"
[[targets]]
name = "mysql-local"
listen = "127.0.0.1:3307"
protocol = "mysql"
database_type = "mysql"
backend_address = "127.0.0.1:3306"
"#,
    )
    .expect("target config should load");

    config
        .apply_env_overrides_from([
            (SQL_LENS_PROXY_LISTEN, "127.0.0.1:5507"),
            (SQL_LENS_BACKEND_ADDRESS, "127.0.0.1:23306"),
        ])
        .expect("valid overrides should apply");

    assert_eq!(
        config.effective_targets(),
        vec![ProxyTargetConfig {
            name: "mysql-local".to_owned(),
            listen: "127.0.0.1:3307".to_owned(),
            protocol: Protocol::MySql,
            database_type: DatabaseType::MySql,
            backend_address: "127.0.0.1:3306".to_owned(),
        }]
    );
}

#[test]
fn invalid_toml_string_returns_parse_error_without_path() {
    let error = SqlLensConfig::from_toml_str("[proxy").expect_err("invalid TOML should fail");

    assert!(matches!(error, ConfigLoadError::Parse { path: None, .. }));
}

#[test]
fn invalid_toml_file_returns_parse_error_with_path() {
    let config_file = temp_config_file("[proxy");

    let error = SqlLensConfig::from_path(&config_file.path).expect_err("invalid TOML should fail");

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

#[test]
fn auth_toml_section_is_rejected() {
    let error = SqlLensConfig::from_toml_str(
        r#"
[auth]
enabled = true
"#,
    )
    .expect_err("application auth config is out of scope");

    assert!(matches!(error, ConfigLoadError::Parse { path: None, .. }));
}
