use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
const STARTUP_CHECK_LOG_MESSAGE: &str = "SQL Lens startup checks completed";

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let id = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sql-lens-app-test-{}-{id}", std::process::id()));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");

        Self { path }
    }

    fn write_config(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("write config file");
        path
    }

    fn missing_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_sql_lens(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sql-lens"))
        .args(args)
        .output()
        .expect("run sql-lens binary")
}

fn valid_config(level: &str, format: &str) -> String {
    format!(
        r#"
[proxy]
listen = "127.0.0.1:4407"
protocol = "mysql"

[backend]
address = "127.0.0.1:3306"

[logging]
level = "{level}"
format = "{format}"
"#
    )
}

fn output_stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

#[test]
fn version_output_succeeds() {
    let output = run_sql_lens(&["--version"]);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("version stdout should be UTF-8");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn valid_config_path_succeeds() {
    let temp_dir = TempDir::new();
    let config_path = temp_dir.write_config("valid.toml", &valid_config("info", "json"));

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = output_stderr(&output);
    assert!(stderr.trim_start().starts_with('{'));
    assert!(stderr.contains("\"level\""));
    assert!(stderr.contains("\"fields\""));
    assert!(stderr.contains(STARTUP_CHECK_LOG_MESSAGE));
}

#[test]
fn pretty_logging_format_emits_human_readable_startup_log() {
    let temp_dir = TempDir::new();
    let config_path = temp_dir.write_config("pretty.toml", &valid_config("info", "pretty"));

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = output_stderr(&output);
    assert!(!stderr.trim_start().starts_with('{'));
    assert!(stderr.contains(STARTUP_CHECK_LOG_MESSAGE));
}

#[test]
fn logging_level_filters_startup_info_event() {
    let temp_dir = TempDir::new();
    let config_path = temp_dir.write_config("error-level.toml", &valid_config("error", "json"));

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = output_stderr(&output);
    assert!(!stderr.contains(STARTUP_CHECK_LOG_MESSAGE));
}

#[test]
fn missing_config_path_fails() {
    let temp_dir = TempDir::new();
    let config_path = temp_dir.missing_path("missing.toml");

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(!output.status.success());

    let stderr = output_stderr(&output);
    assert!(stderr.contains("failed to load SQL Lens config"));
    assert!(stderr.contains("failed to read config file"));
    assert!(stderr.contains("missing.toml"));
}

#[test]
fn invalid_config_validation_fails() {
    let temp_dir = TempDir::new();
    let config_path = temp_dir.write_config(
        "invalid.toml",
        r#"
[proxy]
listen = " "
protocol = "postgresql"

[backend]
address = ""
"#,
    );

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(!output.status.success());

    let stderr = output_stderr(&output);
    assert!(stderr.contains("failed to validate SQL Lens config"));
    assert!(stderr.contains("proxy.listen"));
    assert!(stderr.contains("backend.address"));
    assert!(stderr.contains("proxy.protocol"));
}
