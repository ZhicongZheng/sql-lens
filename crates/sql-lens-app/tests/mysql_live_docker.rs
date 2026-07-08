use std::{error::Error, time::Duration};

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_api::{
    SQL_EVENT_DETAIL_PATH, SQL_EVENTS_PATH, SqlEventDetailResponse, SqlEventListResponse,
    SqlEventSummaryResponse, SqlParameterValueDataResponse,
};
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use tokio::time::{Instant, sleep};

const DOCKER_TESTS_ENV: &str = "SQL_LENS_DOCKER_TESTS";
const MYSQL_PORT: u16 = 3306;
const MYSQL_ROOT_PASSWORD: &str = "sql_lens_root";
const MYSQL_DATABASE: &str = "sql_lens_test";
const PROXY_QUERY: &str = "DO 1";
const PREPARED_UPDATE_SQL: &str = "UPDATE prepared_users SET name = ?, password = ? WHERE id = 42";

#[tokio::test]
async fn docker_mysql_query_is_captured_through_proxy_and_api() -> Result<(), Box<dyn Error>> {
    if std::env::var_os(DOCKER_TESTS_ENV).is_none() {
        eprintln!("skipping Docker MySQL integration test; set {DOCKER_TESTS_ENV}=1 to run");
        return Ok(());
    }

    let mysql = GenericImage::new("mysql", "8.0")
        .with_exposed_port(MYSQL_PORT.tcp())
        .with_env_var("MYSQL_ROOT_PASSWORD", MYSQL_ROOT_PASSWORD)
        .with_env_var("MYSQL_DATABASE", MYSQL_DATABASE)
        .with_cmd(["--default-authentication-plugin=mysql_native_password"])
        .with_startup_timeout(Duration::from_secs(90))
        .start()
        .await?;
    let mysql_host = mysql.get_host().await?;
    let mysql_port = mysql.get_host_port_ipv4(MYSQL_PORT.tcp()).await?;
    let backend_addr = format!("{mysql_host}:{mysql_port}");
    wait_for_mysql_ready(&backend_addr).await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string());
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(PROXY_QUERY).await?;
    conn.disconnect().await?;

    let event = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "query"
            && event.status == "ok"
            && event.original_sql == PROXY_QUERY
    })
    .await?;
    assert_eq!(event.protocol, "mysql");
    assert_eq!(event.kind, "query");
    assert_eq!(event.status, "ok");
    assert_eq!(event.original_sql, PROXY_QUERY);

    runtime.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn docker_mysql_prepared_statement_capture_redacts_via_api() -> Result<(), Box<dyn Error>> {
    if std::env::var_os(DOCKER_TESTS_ENV).is_none() {
        eprintln!("skipping Docker MySQL integration test; set {DOCKER_TESTS_ENV}=1 to run");
        return Ok(());
    }

    let mysql = GenericImage::new("mysql", "8.0")
        .with_exposed_port(MYSQL_PORT.tcp())
        .with_env_var("MYSQL_ROOT_PASSWORD", MYSQL_ROOT_PASSWORD)
        .with_env_var("MYSQL_DATABASE", MYSQL_DATABASE)
        .with_cmd(["--default-authentication-plugin=mysql_native_password"])
        .with_startup_timeout(Duration::from_secs(90))
        .start()
        .await?;
    let mysql_host = mysql.get_host().await?;
    let mysql_port = mysql.get_host_port_ipv4(MYSQL_PORT.tcp()).await?;
    let backend_addr = format!("{mysql_host}:{mysql_port}");
    wait_for_mysql_ready(&backend_addr).await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string());
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(
        "CREATE TABLE prepared_users (
            id INT PRIMARY KEY,
            name VARCHAR(64) NOT NULL,
            password VARCHAR(64) NOT NULL
        )",
    )
    .await?;
    conn.query_drop("INSERT INTO prepared_users (id, name, password) VALUES (42, 'before', 'old')")
        .await?;
    let stmt = conn.prep(PREPARED_UPDATE_SQL).await?;
    conn.exec_drop(&stmt, ("alice", "s3cr3t")).await?;
    conn.close(stmt).await?;
    conn.disconnect().await?;

    let summary = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "statement_execute"
            && event.status == "ok"
            && event.original_sql == PREPARED_UPDATE_SQL
    })
    .await?;
    assert_eq!(
        summary.expanded_sql.as_deref(),
        Some("UPDATE prepared_users SET name = 'alice', password = '***' WHERE id = 42")
    );
    let detail = get_sql_event_detail(runtime.api_addr, &summary.id).await?;

    assert_eq!(detail.protocol, "mysql");
    assert_eq!(detail.kind, "statement_execute");
    assert_eq!(detail.status, "ok");
    assert_eq!(detail.original_sql, PREPARED_UPDATE_SQL);
    assert_eq!(
        detail.expanded_sql.as_deref(),
        Some("UPDATE prepared_users SET name = 'alice', password = '***' WHERE id = 42")
    );
    assert_eq!(detail.parameters.len(), 2);
    assert_eq!(detail.parameters[0].index, 0);
    assert_eq!(detail.parameters[0].name.as_deref(), Some("name"));
    assert_eq!(
        detail.parameters[0].value.value,
        Some(SqlParameterValueDataResponse::String("alice".to_owned()))
    );
    assert!(!detail.parameters[0].redacted);
    assert_eq!(detail.parameters[1].index, 1);
    assert_eq!(detail.parameters[1].name.as_deref(), Some("password"));
    assert_eq!(
        detail.parameters[1].value.value,
        Some(SqlParameterValueDataResponse::String("***".to_owned()))
    );
    assert!(detail.parameters[1].redacted);

    runtime.shutdown().await?;
    Ok(())
}

async fn wait_for_mysql_ready(backend_addr: &str) -> Result<(), Box<dyn Error>> {
    let url = mysql_url(backend_addr);
    let deadline = Instant::now() + Duration::from_secs(90);

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
                eprintln!("waiting for MySQL readiness: {source}");
            }
            Err(source) => return Err(Box::new(source)),
        }

        sleep(Duration::from_millis(500)).await;
    }
}

async fn wait_for_captured_event(
    api_addr: std::net::SocketAddr,
    matches: impl Fn(&SqlEventSummaryResponse) -> bool,
) -> Result<SqlEventSummaryResponse, Box<dyn Error>> {
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

async fn get_sql_event_detail(
    api_addr: std::net::SocketAddr,
    event_id: &str,
) -> Result<SqlEventDetailResponse, Box<dyn Error>> {
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

fn mysql_url(address: &str) -> String {
    format!("mysql://root:{MYSQL_ROOT_PASSWORD}@{address}/{MYSQL_DATABASE}")
}
