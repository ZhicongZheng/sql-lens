use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
    let config_path = temp_dir.write_config(
        "valid.toml",
        r#"
[proxy]
listen = "127.0.0.1:4407"
protocol = "mysql"

[backend]
address = "127.0.0.1:3306"
"#,
    );

    let output = run_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
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

    let stderr = String::from_utf8(output.stderr).expect("error stderr should be UTF-8");
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

    let stderr = String::from_utf8(output.stderr).expect("error stderr should be UTF-8");
    assert!(stderr.contains("failed to validate SQL Lens config"));
    assert!(stderr.contains("proxy.listen"));
    assert!(stderr.contains("backend.address"));
    assert!(stderr.contains("proxy.protocol"));
}
