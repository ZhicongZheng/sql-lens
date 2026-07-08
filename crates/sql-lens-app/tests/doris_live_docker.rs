mod support;

use std::time::Duration;

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

use support::{
    MysqlConnectionOptions, TestResult, mysql_url, shutdown_runtime, skip_unless_env,
    wait_for_captured_event, wait_for_mysql_compatible_ready,
};

const DORIS_TESTS_ENV: &str = "SQL_LENS_DORIS_TESTS";
const DORIS_QUERY_PORT: u16 = 9030;
const DORIS_IMAGE: &str = "apache/doris";
const DORIS_TAG: &str = "2.1.9-all";
const DORIS_SMOKE_QUERY: &str = "CREATE DATABASE IF NOT EXISTS sql_lens_compat_doris";

#[tokio::test]
async fn docker_doris_text_query_is_captured_through_proxy_and_api() -> TestResult {
    if skip_unless_env(DORIS_TESTS_ENV, "Docker Doris") {
        return Ok(());
    }

    let doris = GenericImage::new(DORIS_IMAGE, DORIS_TAG)
        .with_exposed_port(DORIS_QUERY_PORT.tcp())
        .with_startup_timeout(Duration::from_secs(240))
        .start()
        .await?;
    let doris_host = doris.get_host().await?;
    let doris_port = doris.get_host_port_ipv4(DORIS_QUERY_PORT.tcp()).await?;
    let backend_addr = format!("{doris_host}:{doris_port}");
    let options = MysqlConnectionOptions::root();
    wait_for_mysql_compatible_ready("Doris", &backend_addr, options, Duration::from_secs(240))
        .await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string(), options);
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(DORIS_SMOKE_QUERY).await?;
    conn.disconnect().await?;

    let event = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "query"
            && event.status == "ok"
            && event.original_sql == DORIS_SMOKE_QUERY
    })
    .await?;
    assert_eq!(event.protocol, "mysql");
    assert_eq!(event.kind, "query");
    assert_eq!(event.status, "ok");
    assert_eq!(event.original_sql, DORIS_SMOKE_QUERY);

    shutdown_runtime(runtime).await?;
    Ok(())
}
