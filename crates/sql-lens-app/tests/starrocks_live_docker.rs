mod support;

use std::time::Duration;

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

use support::{
    MysqlConnectionOptions, TestResult, mysql_url, shutdown_runtime, skip_unless_env,
    wait_for_captured_event, wait_for_mysql_compatible_ready,
};

const STARROCKS_TESTS_ENV: &str = "SQL_LENS_STARROCKS_TESTS";
const STARROCKS_QUERY_PORT: u16 = 9030;
const STARROCKS_IMAGE: &str = "starrocks/allin1-ubuntu";
const STARROCKS_TAG: &str = "3.5.19";
const STARROCKS_SMOKE_QUERY: &str = "CREATE DATABASE IF NOT EXISTS sql_lens_compat_starrocks";

#[tokio::test]
async fn docker_starrocks_text_query_is_captured_through_proxy_and_api() -> TestResult {
    if skip_unless_env(STARROCKS_TESTS_ENV, "Docker StarRocks") {
        return Ok(());
    }

    let starrocks = GenericImage::new(STARROCKS_IMAGE, STARROCKS_TAG)
        .with_exposed_port(STARROCKS_QUERY_PORT.tcp())
        .with_startup_timeout(Duration::from_secs(180))
        .start()
        .await?;
    let starrocks_host = starrocks.get_host().await?;
    let starrocks_port = starrocks
        .get_host_port_ipv4(STARROCKS_QUERY_PORT.tcp())
        .await?;
    let backend_addr = format!("{starrocks_host}:{starrocks_port}");
    let options = MysqlConnectionOptions::root();
    wait_for_mysql_compatible_ready(
        "StarRocks",
        &backend_addr,
        options,
        Duration::from_secs(180),
    )
    .await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string(), options);
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(STARROCKS_SMOKE_QUERY).await?;
    conn.disconnect().await?;

    let event = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "query"
            && event.status == "ok"
            && event.original_sql == STARROCKS_SMOKE_QUERY
    })
    .await?;
    assert_eq!(event.protocol, "mysql");
    assert_eq!(event.kind, "query");
    assert_eq!(event.status, "ok");
    assert_eq!(event.original_sql, STARROCKS_SMOKE_QUERY);

    shutdown_runtime(runtime).await?;
    Ok(())
}
