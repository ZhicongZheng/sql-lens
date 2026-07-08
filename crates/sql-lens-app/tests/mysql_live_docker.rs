use std::{error::Error, time::Duration};

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_api::{SQL_EVENTS_PATH, SqlEventListResponse};
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use tokio::time::{Instant, sleep};

const DOCKER_TESTS_ENV: &str = "SQL_LENS_DOCKER_TESTS";
const MYSQL_PORT: u16 = 3306;
const MYSQL_ROOT_PASSWORD: &str = "sql_lens_root";
const MYSQL_DATABASE: &str = "sql_lens_test";
const PROXY_QUERY: &str = "DO 1";

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

    let event = wait_for_captured_proxy_query(runtime.api_addr).await?;
    assert_eq!(event.protocol, "mysql");
    assert_eq!(event.kind, "query");
    assert_eq!(event.status, "ok");
    assert_eq!(event.original_sql, PROXY_QUERY);

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

async fn wait_for_captured_proxy_query(
    api_addr: std::net::SocketAddr,
) -> Result<sql_lens_api::SqlEventSummaryResponse, Box<dyn Error>> {
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

        if let Some(event) = response.items.into_iter().find(|event| {
            event.protocol == "mysql"
                && event.kind == "query"
                && event.status == "ok"
                && event.original_sql == PROXY_QUERY
        }) {
            return Ok(event);
        }

        if Instant::now() >= deadline {
            return Err("captured proxy query event did not appear before timeout".into());
        }

        sleep(Duration::from_millis(100)).await;
    }
}

fn mysql_url(address: &str) -> String {
    format!("mysql://root:{MYSQL_ROOT_PASSWORD}@{address}/{MYSQL_DATABASE}")
}
