use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
const STARTUP_CHECK_LOG_MESSAGE: &str = "SQL Lens startup checks completed";
const HEALTH_PATH: &str = "/api/v1/health";

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

fn spawn_sql_lens(args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_sql-lens"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sql-lens binary")
}

fn valid_config(level: &str, format: &str, proxy_addr: SocketAddr, web_addr: SocketAddr) -> String {
    format!(
        r#"
[proxy]
listen = "{proxy_addr}"
protocol = "mysql"

[backend]
address = "127.0.0.1:3306"

[web]
listen = "{web_addr}"

[logging]
level = "{level}"
format = "{format}"
"#
    )
}

fn output_stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn unused_loopback_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral test port");
    listener.local_addr().expect("read ephemeral test port")
}

fn wait_for_health(addr: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        if health_is_ready(addr) {
            return;
        }

        if Instant::now() >= deadline {
            panic!("sql-lens health endpoint did not become ready at {addr}");
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn health_is_ready(addr: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(200)) else {
        return false;
    };

    let request =
        format!("GET {HEALTH_PATH} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && response.starts_with("HTTP/1.1 200")
}

fn stop_process(mut child: Child) -> Output {
    let _ = child.kill();
    child.wait_with_output().expect("wait for sql-lens process")
}

#[test]
fn version_output_succeeds() {
    let output = run_sql_lens(&["--version"]);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("version stdout should be UTF-8");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn valid_config_path_starts_api_server() {
    let temp_dir = TempDir::new();
    let proxy_addr = unused_loopback_addr();
    let web_addr = unused_loopback_addr();
    let config_path = temp_dir.write_config(
        "valid.toml",
        &valid_config("info", "json", proxy_addr, web_addr),
    );

    let child = spawn_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);
    wait_for_health(web_addr);

    let output = stop_process(child);
    assert!(output.stdout.is_empty());

    let stderr = output_stderr(&output);
    assert!(stderr.trim_start().starts_with('{'));
    assert!(stderr.contains("\"level\""));
    assert!(stderr.contains("\"fields\""));
    assert!(stderr.contains(STARTUP_CHECK_LOG_MESSAGE));
    assert!(stderr.contains("SQL Lens API server listening"));
    assert!(stderr.contains("SQL Lens proxy target listening"));
}

#[test]
fn pretty_logging_format_emits_human_readable_startup_log() {
    let temp_dir = TempDir::new();
    let proxy_addr = unused_loopback_addr();
    let web_addr = unused_loopback_addr();
    let config_path = temp_dir.write_config(
        "pretty.toml",
        &valid_config("info", "pretty", proxy_addr, web_addr),
    );

    let child = spawn_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);
    wait_for_health(web_addr);

    let output = stop_process(child);
    assert!(output.stdout.is_empty());

    let stderr = output_stderr(&output);
    assert!(!stderr.trim_start().starts_with('{'));
    assert!(stderr.contains(STARTUP_CHECK_LOG_MESSAGE));
}

#[test]
fn logging_level_filters_startup_info_event() {
    let temp_dir = TempDir::new();
    let proxy_addr = unused_loopback_addr();
    let web_addr = unused_loopback_addr();
    let config_path = temp_dir.write_config(
        "error-level.toml",
        &valid_config("error", "json", proxy_addr, web_addr),
    );

    let child = spawn_sql_lens(&[
        "--config",
        config_path.to_str().expect("path should be UTF-8"),
    ]);
    wait_for_health(web_addr);

    let output = stop_process(child);
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
