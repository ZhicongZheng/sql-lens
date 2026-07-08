mod support;

use std::time::Duration;

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

use support::{
    MysqlConnectionOptions, TestResult, get_sql_event_detail, mysql_url, shutdown_runtime,
    skip_unless_env, wait_for_captured_event, wait_for_mysql_compatible_ready,
};

const TIDB_TESTS_ENV: &str = "SQL_LENS_TIDB_TESTS";
const TIDB_QUERY_PORT: u16 = 4000;
const TIDB_IMAGE: &str = "pingcap/tidb";
const TIDB_TAG: &str = "v8.5.6";
const TIDB_TEXT_QUERY: &str = "CREATE DATABASE IF NOT EXISTS sql_lens_compat_tidb";
const TIDB_PREPARED_SQL: &str =
    "UPDATE sql_lens_compat_tidb.prepared_users SET name = ?, password = ? WHERE id = 42";

#[tokio::test]
async fn docker_tidb_text_and_prepared_queries_are_captured_through_proxy_and_api() -> TestResult {
    if skip_unless_env(TIDB_TESTS_ENV, "Docker TiDB") {
        return Ok(());
    }

    let tidb = GenericImage::new(TIDB_IMAGE, TIDB_TAG)
        .with_exposed_port(TIDB_QUERY_PORT.tcp())
        .with_startup_timeout(Duration::from_secs(120))
        .start()
        .await?;
    let tidb_host = tidb.get_host().await?;
    let tidb_port = tidb.get_host_port_ipv4(TIDB_QUERY_PORT.tcp()).await?;
    let backend_addr = format!("{tidb_host}:{tidb_port}");
    let options = MysqlConnectionOptions::root();
    wait_for_mysql_compatible_ready("TiDB", &backend_addr, options, Duration::from_secs(120))
        .await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string(), options);
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(TIDB_TEXT_QUERY).await?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS sql_lens_compat_tidb.prepared_users (
            id INT PRIMARY KEY,
            name VARCHAR(64) NOT NULL,
            password VARCHAR(64) NOT NULL
        )",
    )
    .await?;
    conn.query_drop(
        "INSERT INTO sql_lens_compat_tidb.prepared_users (id, name, password)
         VALUES (42, 'before', 'old')
         ON DUPLICATE KEY UPDATE name = VALUES(name), password = VALUES(password)",
    )
    .await?;
    let stmt = conn.prep(TIDB_PREPARED_SQL).await?;
    conn.exec_drop(&stmt, ("alice", "s3cr3t")).await?;
    conn.close(stmt).await?;
    conn.disconnect().await?;

    let text_event = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "query"
            && event.status == "ok"
            && event.original_sql == TIDB_TEXT_QUERY
    })
    .await?;
    assert_eq!(text_event.kind, "query");
    assert_eq!(text_event.status, "ok");
    assert_eq!(text_event.original_sql, TIDB_TEXT_QUERY);

    let prepared_summary = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "statement_execute"
            && event.status == "ok"
            && event.original_sql == TIDB_PREPARED_SQL
    })
    .await?;
    assert_eq!(
        prepared_summary.expanded_sql.as_deref(),
        Some(
            "UPDATE sql_lens_compat_tidb.prepared_users SET name = 'alice', password = '***' WHERE id = 42"
        )
    );
    let prepared_detail = get_sql_event_detail(runtime.api_addr, &prepared_summary.id).await?;
    assert_eq!(prepared_detail.protocol, "mysql");
    assert_eq!(prepared_detail.kind, "statement_execute");
    assert_eq!(prepared_detail.status, "ok");
    assert_eq!(prepared_detail.original_sql, TIDB_PREPARED_SQL);
    assert_eq!(prepared_detail.parameters.len(), 2);
    assert_eq!(prepared_detail.parameters[0].name.as_deref(), Some("name"));
    assert_eq!(
        prepared_detail.parameters[1].name.as_deref(),
        Some("password")
    );
    assert!(prepared_detail.parameters[1].redacted);

    shutdown_runtime(runtime).await?;
    Ok(())
}
