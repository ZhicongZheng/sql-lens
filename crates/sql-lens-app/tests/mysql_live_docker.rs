mod support;

use std::time::Duration;

use mysql_async::{Conn, prelude::Queryable};
use sql_lens_api::SqlParameterValueDataResponse;
use sql_lens_app::start_minimal_mysql_runtime;
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

use support::{
    MysqlConnectionOptions, TestResult, get_sql_event_detail, mysql_url, shutdown_runtime,
    skip_unless_env, wait_for_captured_event, wait_for_mysql_compatible_ready,
};

const DOCKER_TESTS_ENV: &str = "SQL_LENS_DOCKER_TESTS";
const MYSQL_PORT: u16 = 3306;
const MYSQL_ROOT_PASSWORD: &str = "sql_lens_root";
const MYSQL_DATABASE: &str = "sql_lens_test";
const PROXY_QUERY: &str = "DO 1";
const PROXY_SELECT_QUERY: &str = "SELECT 1";
const PREPARED_UPDATE_SQL: &str = "UPDATE prepared_users SET name = ?, password = ? WHERE id = 42";

#[tokio::test]
async fn docker_mysql_query_is_captured_through_proxy_and_api() -> TestResult {
    if skip_unless_env(DOCKER_TESTS_ENV, "Docker MySQL") {
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
    let options = MysqlConnectionOptions::root_with_password(MYSQL_ROOT_PASSWORD, MYSQL_DATABASE);
    wait_for_mysql_compatible_ready("MySQL", &backend_addr, options, Duration::from_secs(90))
        .await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string(), options);
    let mut conn = Conn::from_url(proxy_url).await?;
    conn.query_drop(PROXY_QUERY).await?;
    let selected: Option<u8> = conn.query_first(PROXY_SELECT_QUERY).await?;
    assert_eq!(selected, Some(1));
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
    let select_event = wait_for_captured_event(runtime.api_addr, |event| {
        event.protocol == "mysql"
            && event.kind == "query"
            && event.status == "ok"
            && event.original_sql == PROXY_SELECT_QUERY
            && event.rows.and_then(|rows| rows.returned) == Some(1)
    })
    .await?;
    assert_eq!(select_event.protocol, "mysql");
    assert_eq!(select_event.kind, "query");
    assert_eq!(select_event.status, "ok");
    assert_eq!(select_event.original_sql, PROXY_SELECT_QUERY);
    assert_eq!(select_event.rows.and_then(|rows| rows.returned), Some(1));

    shutdown_runtime(runtime).await?;
    Ok(())
}

#[tokio::test]
async fn docker_mysql_prepared_statement_capture_redacts_via_api() -> TestResult {
    if skip_unless_env(DOCKER_TESTS_ENV, "Docker MySQL") {
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
    let options = MysqlConnectionOptions::root_with_password(MYSQL_ROOT_PASSWORD, MYSQL_DATABASE);
    wait_for_mysql_compatible_ready("MySQL", &backend_addr, options, Duration::from_secs(90))
        .await?;
    let runtime = start_minimal_mysql_runtime(backend_addr).await?;

    let proxy_url = mysql_url(&runtime.proxy_addr.to_string(), options);
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

    shutdown_runtime(runtime).await?;
    Ok(())
}
