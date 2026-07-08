use std::{error::Error, net::SocketAddr, time::Duration};

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_api::{
    SQL_EVENT_DETAIL_PATH, SQL_EVENTS_PATH, SqlEventDetailResponse, SqlEventListResponse,
    SqlEventSummaryResponse,
};
use sql_lens_app::MinimalMysqlRuntime;
use tokio::time::{Instant, sleep};

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Copy)]
pub struct MysqlConnectionOptions<'a> {
    pub username: &'a str,
    pub password: Option<&'a str>,
    pub database: Option<&'a str>,
}

impl<'a> MysqlConnectionOptions<'a> {
    #[allow(dead_code)]
    pub fn root() -> Self {
        Self {
            username: "root",
            password: None,
            database: None,
        }
    }

    #[allow(dead_code)]
    pub fn root_with_password(password: &'a str, database: &'a str) -> Self {
        Self {
            username: "root",
            password: Some(password),
            database: Some(database),
        }
    }
}

pub fn skip_unless_env(env_key: &str, label: &str) -> bool {
    if std::env::var_os(env_key).is_some() {
        return false;
    }

    eprintln!("skipping {label} integration test; set {env_key}=1 to run");
    true
}

pub fn mysql_url(address: &str, options: MysqlConnectionOptions<'_>) -> String {
    let credentials = match options.password {
        Some(password) => format!("{}:{password}", options.username),
        None => options.username.to_owned(),
    };
    let database = options
        .database
        .map(|database| format!("/{database}"))
        .unwrap_or_default();

    format!("mysql://{credentials}@{address}{database}?prefer_socket=false")
}

pub async fn wait_for_mysql_compatible_ready(
    label: &str,
    backend_addr: &str,
    options: MysqlConnectionOptions<'_>,
    timeout: Duration,
) -> TestResult {
    let url = mysql_url(backend_addr, options);
    let deadline = Instant::now() + timeout;

    loop {
        match Conn::from_url(url.as_str()).await {
            Ok(mut conn) => {
                let selected: Option<u8> = conn.query_first("SELECT 1").await?;
                conn.disconnect().await?;
                if selected == Some(1) {
                    return Ok(());
                }
            }
            Err(source) if Instant::now() < deadline => {
                eprintln!("waiting for {label} readiness: {source}");
            }
            Err(source) => return Err(Box::new(source)),
        }

        sleep(Duration::from_millis(500)).await;
    }
}

pub async fn wait_for_captured_event(
    api_addr: SocketAddr,
    matches: impl Fn(&SqlEventSummaryResponse) -> bool,
) -> TestResult<SqlEventSummaryResponse> {
    let client = reqwest::Client::new();
    let url = format!("http://{api_addr}{SQL_EVENTS_PATH}");
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        let response = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<SqlEventListResponse>()
            .await?;

        if let Some(event) = response.items.into_iter().find(|event| matches(event)) {
            return Ok(event);
        }

        if Instant::now() >= deadline {
            return Err("captured proxy event did not appear before timeout".into());
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[allow(dead_code)]
pub async fn get_sql_event_detail(
    api_addr: SocketAddr,
    event_id: &str,
) -> TestResult<SqlEventDetailResponse> {
    let client = reqwest::Client::new();
    let path = SQL_EVENT_DETAIL_PATH.replace("{id}", event_id);
    let url = format!("http://{api_addr}{path}");
    let detail = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<SqlEventDetailResponse>()
        .await?;

    Ok(detail)
}

pub async fn shutdown_runtime(runtime: MinimalMysqlRuntime) -> TestResult {
    runtime.shutdown().await?;
    Ok(())
}
