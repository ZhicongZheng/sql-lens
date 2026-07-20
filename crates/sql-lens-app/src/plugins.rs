//! Plugin loading and isolated hook dispatch for the SQL Lens runtime.
//!
//! # Loading boundary
//!
//! Plugins are loaded only when `plugins.enabled` is true. The configured
//! `plugins.directory` is scanned for **`.toml` manifests only**; other files
//! (README, `.gitkeep`, binaries, etc.) are skipped and do not fail startup.
//!
//! Each manifest must look like:
//!
//! ```toml
//! name = "my-plugin"
//! kind = "builtin_noop"
//! ```
//!
//! Supported production kinds today:
//! - `builtin_noop` — statically registered no-op hook implementation
//!
//! Native / dynamic shared-library loading is intentionally unsupported. Tests
//! may inject plugin instances via [`PluginRuntime::start_with_plugins`] without
//! going through the filesystem. `plugins.allow_network` is accepted as config
//! but has no effect until network-capable plugins exist.
//!
//! # Failure isolation
//!
//! Dispatch runs on an async worker off the proxy hot path. Each plugin hook is
//! executed in `spawn_blocking` under `plugins.timeout_ms`. Failures, panics,
//! and timeouts are logged and do not propagate to packet forwarding or capture
//! delivery. Queue overflow drops plugin payloads with a warning.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Deserialize;
use sql_lens_config::PluginsConfig;
use sql_lens_core::{
    ConnectionInfo, ErrorSummary, PreparedStatementInfo, RedactionPolicy, SqlEvent, SqlEventKind,
    redact_sql_event,
};
use sql_lens_plugin::{
    OnConnect, OnError, OnExecute, OnPrepare, OnQuery, PluginError, PluginResult,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

const PLUGIN_QUEUE_CAPACITY: usize = 256;
const SUPPORTED_KIND_BUILTIN_NOOP: &str = "builtin_noop";

type LoadedPlugins = Vec<(String, Arc<Mutex<Box<dyn PluginInstance>>>)>;

#[derive(Debug, Clone)]
pub(crate) struct PluginRuntimeHandle {
    sender: mpsc::Sender<PluginPayload>,
    redaction_policy: RedactionPolicy,
}

impl PluginRuntimeHandle {
    pub(crate) fn dispatch_connect(&self, connection: ConnectionInfo) {
        self.try_send(PluginPayload::Connect(Box::new(connection)));
    }

    pub(crate) fn dispatch_event(&self, event: SqlEvent) {
        let event = redact_sql_event(event, &self.redaction_policy);
        let connection = connection_from_event(&event);
        self.try_send(PluginPayload::Event {
            event: Box::new(event),
            connection: Box::new(connection),
        });
    }

    fn try_send(&self, payload: PluginPayload) {
        if self.sender.try_send(payload).is_err() {
            tracing::warn!("plugin queue is full or stopped; dropping plugin payload");
        }
    }
}

#[derive(Debug)]
pub(crate) struct PluginRuntime {
    handle: PluginRuntimeHandle,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl Drop for PluginRuntime {
    fn drop(&mut self) {
        // Do not leave the plugin worker detached if startup fails before the
        // runtime is returned to its caller.
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl PluginRuntime {
    pub(crate) fn start(
        config: &PluginsConfig,
        redaction_policy: RedactionPolicy,
    ) -> Result<Option<Self>, PluginRuntimeError> {
        if !config.enabled {
            tracing::info!("plugin runtime disabled");
            return Ok(None);
        }
        validate_plugins_config(config)?;

        let plugins = load_plugins(Path::new(&config.directory))?;
        Ok(Some(Self::spawn_worker(
            plugins,
            redaction_policy,
            Duration::from_millis(config.timeout_ms),
        )))
    }

    /// Start a plugin runtime with pre-built plugin instances (tests / injection).
    ///
    /// Plugins are invoked in the order given.
    #[cfg(test)]
    pub(crate) fn start_with_plugins(
        plugins: Vec<(String, Box<dyn PluginInstance>)>,
        redaction_policy: RedactionPolicy,
        timeout: Duration,
    ) -> Result<Self, PluginRuntimeError> {
        if timeout.is_zero() {
            return Err(PluginRuntimeError::InvalidConfig(
                "plugins.timeout_ms must be greater than zero".to_owned(),
            ));
        }
        let plugins = plugins
            .into_iter()
            .map(|(name, plugin)| (name, Arc::new(Mutex::new(plugin))))
            .collect();
        Ok(Self::spawn_worker(plugins, redaction_policy, timeout))
    }

    fn spawn_worker(
        plugins: LoadedPlugins,
        redaction_policy: RedactionPolicy,
        timeout: Duration,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(PLUGIN_QUEUE_CAPACITY);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(run_plugin_worker(receiver, shutdown_rx, plugins, timeout));

        Self {
            handle: PluginRuntimeHandle {
                sender,
                redaction_policy,
            },
            shutdown_tx: Some(shutdown_tx),
            task: Some(task),
        }
    }

    pub(crate) fn handle(&self) -> PluginRuntimeHandle {
        self.handle.clone()
    }

    pub(crate) async fn shutdown(mut self) -> Result<(), PluginRuntimeError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(task) = self.task.take() {
            task.await
                .map_err(|error| PluginRuntimeError::Worker(error.to_string()))?;
        }
        Ok(())
    }
}

fn validate_plugins_config(config: &PluginsConfig) -> Result<(), PluginRuntimeError> {
    if config.timeout_ms == 0 {
        return Err(PluginRuntimeError::InvalidConfig(
            "plugins.timeout_ms must be greater than zero".to_owned(),
        ));
    }
    // allow_network is accepted without effect until network-capable plugins exist.
    let _ = config.allow_network;
    Ok(())
}

#[derive(Debug)]
enum PluginPayload {
    Connect(Box<ConnectionInfo>),
    Event {
        event: Box<SqlEvent>,
        connection: Box<ConnectionInfo>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginManifest {
    name: String,
    kind: String,
}

/// Object-safe runtime plugin surface used by the dispatcher.
pub(crate) trait PluginInstance: Send {
    fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult;
    fn on_query(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
    fn on_prepare(
        &mut self,
        statement: &PreparedStatementInfo,
        connection: &ConnectionInfo,
    ) -> PluginResult;
    fn on_execute(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
    fn on_error(
        &mut self,
        event: &SqlEvent,
        connection: &ConnectionInfo,
        error: &ErrorSummary,
    ) -> PluginResult;
}

#[derive(Debug, Default)]
struct BuiltinNoopPlugin;

impl OnConnect for BuiltinNoopPlugin {
    fn on_connect(&mut self, _: &ConnectionInfo) -> PluginResult {
        Ok(())
    }
}

impl OnQuery for BuiltinNoopPlugin {
    fn on_query(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
        Ok(())
    }
}

impl OnPrepare for BuiltinNoopPlugin {
    fn on_prepare(&mut self, _: &PreparedStatementInfo, _: &ConnectionInfo) -> PluginResult {
        Ok(())
    }
}

impl OnExecute for BuiltinNoopPlugin {
    fn on_execute(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
        Ok(())
    }
}

impl OnError for BuiltinNoopPlugin {
    fn on_error(&mut self, _: &SqlEvent, _: &ConnectionInfo, _: &ErrorSummary) -> PluginResult {
        Ok(())
    }
}

impl PluginInstance for BuiltinNoopPlugin {
    fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult {
        OnConnect::on_connect(self, connection)
    }

    fn on_query(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult {
        OnQuery::on_query(self, event, connection)
    }

    fn on_prepare(
        &mut self,
        statement: &PreparedStatementInfo,
        connection: &ConnectionInfo,
    ) -> PluginResult {
        OnPrepare::on_prepare(self, statement, connection)
    }

    fn on_execute(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult {
        OnExecute::on_execute(self, event, connection)
    }

    fn on_error(
        &mut self,
        event: &SqlEvent,
        connection: &ConnectionInfo,
        error: &ErrorSummary,
    ) -> PluginResult {
        OnError::on_error(self, event, connection, error)
    }
}

async fn run_plugin_worker(
    mut receiver: mpsc::Receiver<PluginPayload>,
    mut shutdown: oneshot::Receiver<()>,
    plugins: LoadedPlugins,
    timeout: Duration,
) {
    loop {
        tokio::select! {
            payload = receiver.recv() => {
                let Some(payload) = payload else { return; };
                dispatch_payload(&plugins, payload, timeout).await;
            }
            _ = &mut shutdown => {
                while let Ok(payload) = receiver.try_recv() {
                    dispatch_payload(&plugins, payload, timeout).await;
                }
                return;
            }
        }
    }
}

async fn dispatch_payload(plugins: &LoadedPlugins, payload: PluginPayload, timeout: Duration) {
    for (name, plugin) in plugins {
        let plugin = Arc::clone(plugin);
        let payload = clone_payload(&payload);
        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut plugin = plugin
                    .lock()
                    .map_err(|_| PluginError::hook_failed("plugin lock poisoned"))?;
                dispatch_to_plugin(&mut **plugin, payload)
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => {
                tracing::warn!(plugin = %name, error = %error, "plugin hook failed")
            }
            Ok(Err(error)) => {
                tracing::warn!(plugin = %name, error = %error, "plugin worker panicked")
            }
            Err(_) => tracing::warn!(plugin = %name, "plugin hook timed out"),
        }
    }
}

fn dispatch_to_plugin(plugin: &mut dyn PluginInstance, payload: PluginPayload) -> PluginResult {
    match payload {
        PluginPayload::Connect(connection) => plugin.on_connect(&connection),
        PluginPayload::Event { event, connection } => {
            if event.kind == SqlEventKind::Query {
                plugin.on_query(&event, &connection)?;
            }
            if event.kind == SqlEventKind::StatementPrepare {
                plugin.on_prepare(&statement_from_event(&event), &connection)?;
            }
            if event.kind == SqlEventKind::StatementExecute {
                plugin.on_execute(&event, &connection)?;
            }
            if let Some(error) = &event.error {
                plugin.on_error(&event, &connection, error)?;
            }
            Ok(())
        }
    }
}

fn clone_payload(payload: &PluginPayload) -> PluginPayload {
    match payload {
        PluginPayload::Connect(connection) => PluginPayload::Connect(connection.clone()),
        PluginPayload::Event { event, connection } => PluginPayload::Event {
            event: event.clone(),
            connection: connection.clone(),
        },
    }
}

fn load_plugins(directory: &Path) -> Result<LoadedPlugins, PluginRuntimeError> {
    let entries = fs::read_dir(directory).map_err(|source| PluginRuntimeError::Directory {
        path: directory.to_path_buf(),
        source: source.to_string(),
    })?;
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| PluginRuntimeError::Directory {
            path: directory.to_path_buf(),
            source: source.to_string(),
        })?;
    // Stable dispatch order: sorted filesystem path.
    paths.sort();

    let mut plugins = Vec::new();
    for path in paths {
        if !path.is_file() {
            continue;
        }
        // Non-TOML entries are ignored so README / lockfiles do not fail startup.
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }
        let contents =
            fs::read_to_string(&path).map_err(|source| PluginRuntimeError::Artifact {
                path: path.clone(),
                message: source.to_string(),
            })?;
        let manifest: PluginManifest =
            toml::from_str(&contents).map_err(|source| PluginRuntimeError::Artifact {
                path: path.clone(),
                message: source.to_string(),
            })?;
        if manifest.name.trim().is_empty() {
            return Err(PluginRuntimeError::Artifact {
                path: path.clone(),
                message: "manifest name must not be empty".to_owned(),
            });
        }
        if manifest.kind != SUPPORTED_KIND_BUILTIN_NOOP {
            return Err(PluginRuntimeError::Artifact {
                path,
                message: format!(
                    "unsupported plugin kind {:?}; only \"{SUPPORTED_KIND_BUILTIN_NOOP}\" is supported",
                    manifest.kind
                ),
            });
        }
        plugins.push((
            manifest.name,
            Arc::new(Mutex::new(
                Box::new(BuiltinNoopPlugin) as Box<dyn PluginInstance>
            )),
        ));
    }

    Ok(plugins)
}

fn connection_from_event(event: &SqlEvent) -> ConnectionInfo {
    ConnectionInfo {
        id: event.connection_id.clone(),
        target_name: event.target_name.clone(),
        protocol: event.protocol.clone(),
        database_type: event.database_type.clone(),
        client_addr: event.client_addr.clone(),
        backend_addr: event.backend_addr.clone(),
        user: event.user.clone(),
        database: event.database.clone(),
        state: sql_lens_core::ConnectionState::Ready,
        connected_at: event.timings.started_at.clone(),
        closed_at: None,
        last_activity_at: Some(event.timestamp.clone()),
        bytes_in: 0,
        bytes_out: 0,
        query_count: 0,
    }
}

fn statement_from_event(event: &SqlEvent) -> PreparedStatementInfo {
    PreparedStatementInfo {
        connection_id: event.connection_id.clone(),
        statement_id: sql_lens_core::StatementId(event.id.0.clone()),
        protocol: event.protocol.clone(),
        template_sql: event.original_sql.clone(),
        parameter_count: event.parameters.len().min(u16::MAX as usize) as u16,
        created_at: event.timestamp.clone(),
        closed_at: None,
        metadata: event.metadata.clone(),
    }
}

#[derive(Debug)]
pub enum PluginRuntimeError {
    InvalidConfig(String),
    Directory { path: PathBuf, source: String },
    Artifact { path: PathBuf, message: String },
    Worker(String),
}

impl std::fmt::Display for PluginRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid plugin config: {message}"),
            Self::Directory { path, source } => {
                write!(
                    formatter,
                    "failed to read plugin directory {}: {source}",
                    path.display()
                )
            }
            Self::Artifact { path, message } => {
                write!(
                    formatter,
                    "invalid plugin artifact {}: {message}",
                    path.display()
                )
            }
            Self::Worker(message) => write!(formatter, "plugin worker failed: {message}"),
        }
    }
}

impl std::error::Error for PluginRuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, ConnectionState, DEFAULT_REDACTION_MASK, DatabaseType,
        DurationMillis, ErrorSummary, ProtocolMetadata, ProtocolName, QueryTiming, RedactionPolicy,
        SqlEvent, SqlEventId, SqlEventKind, SqlParameter, SqlParameterValue, Timestamp,
    };
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::Duration,
    };

    #[derive(Default)]
    struct RecordingPlugin {
        calls: Arc<Mutex<Vec<String>>>,
        query_sql: Arc<Mutex<Option<String>>>,
        prepare_sql: Arc<Mutex<Option<String>>>,
        execute_sql: Arc<Mutex<Option<String>>>,
        error_message: Arc<Mutex<Option<String>>>,
        connect_id: Arc<Mutex<Option<String>>>,
    }

    #[derive(Clone)]
    struct RecordingHandles {
        calls: Arc<Mutex<Vec<String>>>,
        query_sql: Arc<Mutex<Option<String>>>,
        prepare_sql: Arc<Mutex<Option<String>>>,
        execute_sql: Arc<Mutex<Option<String>>>,
        error_message: Arc<Mutex<Option<String>>>,
        connect_id: Arc<Mutex<Option<String>>>,
    }

    impl RecordingPlugin {
        fn shared() -> (Self, RecordingHandles) {
            let plugin = Self::default();
            let handles = RecordingHandles {
                calls: Arc::clone(&plugin.calls),
                query_sql: Arc::clone(&plugin.query_sql),
                prepare_sql: Arc::clone(&plugin.prepare_sql),
                execute_sql: Arc::clone(&plugin.execute_sql),
                error_message: Arc::clone(&plugin.error_message),
                connect_id: Arc::clone(&plugin.connect_id),
            };
            (
                Self {
                    calls: Arc::clone(&handles.calls),
                    query_sql: Arc::clone(&handles.query_sql),
                    prepare_sql: Arc::clone(&handles.prepare_sql),
                    execute_sql: Arc::clone(&handles.execute_sql),
                    error_message: Arc::clone(&handles.error_message),
                    connect_id: Arc::clone(&handles.connect_id),
                },
                handles,
            )
        }
    }

    impl PluginInstance for RecordingPlugin {
        fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult {
            self.calls.lock().expect("lock").push("connect".to_owned());
            *self.connect_id.lock().expect("lock") = Some(connection.id.0.clone());
            Ok(())
        }

        fn on_query(&mut self, event: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            self.calls.lock().expect("lock").push("query".to_owned());
            *self.query_sql.lock().expect("lock") = Some(event.original_sql.clone());
            Ok(())
        }

        fn on_prepare(
            &mut self,
            statement: &PreparedStatementInfo,
            _: &ConnectionInfo,
        ) -> PluginResult {
            self.calls.lock().expect("lock").push("prepare".to_owned());
            *self.prepare_sql.lock().expect("lock") = Some(statement.template_sql.clone());
            Ok(())
        }

        fn on_execute(&mut self, event: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            self.calls.lock().expect("lock").push("execute".to_owned());
            *self.execute_sql.lock().expect("lock") = Some(event.original_sql.clone());
            Ok(())
        }

        fn on_error(
            &mut self,
            _: &SqlEvent,
            _: &ConnectionInfo,
            error: &ErrorSummary,
        ) -> PluginResult {
            self.calls.lock().expect("lock").push("error".to_owned());
            *self.error_message.lock().expect("lock") = Some(error.message.clone());
            Ok(())
        }
    }

    struct FailingPlugin;

    impl PluginInstance for FailingPlugin {
        fn on_connect(&mut self, _: &ConnectionInfo) -> PluginResult {
            Err(PluginError::hook_failed("connect boom"))
        }

        fn on_query(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            Err(PluginError::hook_failed("query boom"))
        }

        fn on_prepare(&mut self, _: &PreparedStatementInfo, _: &ConnectionInfo) -> PluginResult {
            Err(PluginError::hook_failed("prepare boom"))
        }

        fn on_execute(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            Err(PluginError::hook_failed("execute boom"))
        }

        fn on_error(&mut self, _: &SqlEvent, _: &ConnectionInfo, _: &ErrorSummary) -> PluginResult {
            Err(PluginError::hook_failed("error boom"))
        }
    }

    struct SlowPlugin {
        delay: Duration,
        started: Arc<AtomicUsize>,
    }

    impl PluginInstance for SlowPlugin {
        fn on_connect(&mut self, _: &ConnectionInfo) -> PluginResult {
            self.started.fetch_add(1, Ordering::SeqCst);
            thread::sleep(self.delay);
            Ok(())
        }

        fn on_query(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            self.started.fetch_add(1, Ordering::SeqCst);
            thread::sleep(self.delay);
            Ok(())
        }

        fn on_prepare(&mut self, _: &PreparedStatementInfo, _: &ConnectionInfo) -> PluginResult {
            Ok(())
        }

        fn on_execute(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            Ok(())
        }

        fn on_error(&mut self, _: &SqlEvent, _: &ConnectionInfo, _: &ErrorSummary) -> PluginResult {
            Ok(())
        }
    }

    struct OrderPlugin {
        name: String,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl PluginInstance for OrderPlugin {
        fn on_connect(&mut self, _: &ConnectionInfo) -> PluginResult {
            self.order.lock().expect("lock").push(self.name.clone());
            Ok(())
        }

        fn on_query(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            self.order.lock().expect("lock").push(self.name.clone());
            Ok(())
        }

        fn on_prepare(&mut self, _: &PreparedStatementInfo, _: &ConnectionInfo) -> PluginResult {
            Ok(())
        }

        fn on_execute(&mut self, _: &SqlEvent, _: &ConnectionInfo) -> PluginResult {
            Ok(())
        }

        fn on_error(&mut self, _: &SqlEvent, _: &ConnectionInfo, _: &ErrorSummary) -> PluginResult {
            Ok(())
        }
    }

    #[test]
    fn disabled_plugins_do_not_load_or_start() {
        let config = PluginsConfig {
            enabled: false,
            directory: "/definitely/missing/plugins".to_owned(),
            allow_network: false,
            timeout_ms: 0,
        };
        let runtime = PluginRuntime::start(&config, RedactionPolicy::default())
            .expect("disabled plugins should not validate directory or timeout");
        assert!(runtime.is_none());
    }

    #[test]
    fn zero_timeout_is_rejected_when_enabled() {
        let config = PluginsConfig {
            enabled: true,
            directory: "plugins".to_owned(),
            allow_network: false,
            timeout_ms: 0,
        };
        let error = PluginRuntime::start(&config, RedactionPolicy::default())
            .expect_err("timeout_ms=0 must fail at startup");
        assert!(error.to_string().contains("timeout_ms"));
    }

    #[test]
    fn missing_directory_returns_clear_startup_error() {
        let config = PluginsConfig {
            enabled: true,
            directory: "/definitely/missing/sql-lens-plugins".to_owned(),
            allow_network: false,
            timeout_ms: 50,
        };
        let error = PluginRuntime::start(&config, RedactionPolicy::default())
            .expect_err("missing directory must fail when enabled");
        assert!(
            error.to_string().contains("plugin directory"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn malformed_manifest_and_unknown_kind_return_clear_errors() {
        let root = unique_temp_dir("plugin-manifest-errors");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(root.join("README.md"), "ignore me").expect("write readme");
        fs::write(root.join("bad.toml"), "not = [valid").expect("write bad toml");

        let error = match load_plugins(&root) {
            Ok(_) => panic!("bad toml must fail"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("invalid plugin artifact"),
            "unexpected error: {error}"
        );

        fs::remove_file(root.join("bad.toml")).expect("remove bad");
        fs::write(
            root.join("unknown.toml"),
            "name = \"x\"\nkind = \"native_so\"\n",
        )
        .expect("write unknown");
        let error = match load_plugins(&root) {
            Ok(_) => panic!("unknown kind must fail"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("unsupported plugin kind"),
            "unexpected error: {error}"
        );

        fs::remove_file(root.join("unknown.toml")).expect("remove unknown");
        fs::write(
            root.join("empty.toml"),
            "name = \"\"\nkind = \"builtin_noop\"\n",
        )
        .expect("write empty name");
        let error = match load_plugins(&root) {
            Ok(_) => panic!("empty name must fail"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("name must not be empty"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn loads_builtin_noop_manifests_in_sorted_path_order_and_skips_non_toml() {
        let root = unique_temp_dir("plugin-load-order");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(root.join("README.md"), "skip").expect("readme");
        fs::write(
            root.join("b_second.toml"),
            "name = \"second\"\nkind = \"builtin_noop\"\n",
        )
        .expect("second");
        fs::write(
            root.join("a_first.toml"),
            "name = \"first\"\nkind = \"builtin_noop\"\n",
        )
        .expect("first");

        let plugins = load_plugins(&root).expect("valid manifests should load");
        let names = plugins
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["first", "second"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn recording_plugin_receives_redacted_hook_payloads() {
        let (plugin, handles) = RecordingPlugin::shared();
        let runtime = PluginRuntime::start_with_plugins(
            vec![("recorder".to_owned(), Box::new(plugin))],
            RedactionPolicy::default(),
            Duration::from_millis(200),
        )
        .expect("runtime should start");
        let handle = runtime.handle();

        handle.dispatch_connect(sample_connection());
        handle.dispatch_event(sample_event(SqlEventKind::Query, false));
        handle.dispatch_event(sample_event(SqlEventKind::StatementPrepare, false));
        handle.dispatch_event(sample_event(SqlEventKind::StatementExecute, false));
        handle.dispatch_event(sample_event(SqlEventKind::Query, true));

        wait_for(|| handles.calls.lock().expect("lock").len() >= 6).await;
        runtime.shutdown().await.expect("shutdown should succeed");

        assert_eq!(
            *handles.calls.lock().expect("lock"),
            ["connect", "query", "prepare", "execute", "query", "error"]
        );
        assert_eq!(
            handles.connect_id.lock().expect("lock").as_deref(),
            Some("conn_plugin")
        );
        // Default redaction policy masks password-like parameters in SQL text.
        let query = handles
            .query_sql
            .lock()
            .expect("lock")
            .clone()
            .expect("query sql");
        assert!(
            query.contains(DEFAULT_REDACTION_MASK) || !query.contains("secret-password"),
            "query payload should be redacted: {query}"
        );
        let prepare = handles
            .prepare_sql
            .lock()
            .expect("lock")
            .clone()
            .expect("prepare sql");
        assert!(
            prepare.contains(DEFAULT_REDACTION_MASK) || !prepare.contains("secret-password"),
            "prepare payload should be redacted: {prepare}"
        );
        let execute = handles
            .execute_sql
            .lock()
            .expect("lock")
            .clone()
            .expect("execute sql");
        assert!(
            execute.contains(DEFAULT_REDACTION_MASK) || !execute.contains("secret-password"),
            "execute payload should be redacted: {execute}"
        );
        assert_eq!(
            handles.error_message.lock().expect("lock").as_deref(),
            Some("boom")
        );
    }

    #[tokio::test]
    async fn failing_plugin_is_isolated_and_other_plugins_continue() {
        let (plugin, handles) = RecordingPlugin::shared();
        let runtime = PluginRuntime::start_with_plugins(
            vec![
                ("failing".to_owned(), Box::new(FailingPlugin)),
                ("recorder".to_owned(), Box::new(plugin)),
            ],
            RedactionPolicy {
                enabled: false,
                ..RedactionPolicy::default()
            },
            Duration::from_millis(200),
        )
        .expect("runtime should start");
        let handle = runtime.handle();
        handle.dispatch_event(sample_event(SqlEventKind::Query, false));

        wait_for(|| {
            handles
                .calls
                .lock()
                .expect("lock")
                .contains(&"query".to_owned())
        })
        .await;
        runtime.shutdown().await.expect("shutdown should succeed");
        assert!(
            handles
                .calls
                .lock()
                .expect("lock")
                .contains(&"query".to_owned())
        );
    }

    #[tokio::test]
    async fn timing_out_plugin_is_isolated() {
        let started = Arc::new(AtomicUsize::new(0));
        let (plugin, handles) = RecordingPlugin::shared();
        let runtime = PluginRuntime::start_with_plugins(
            vec![
                (
                    "slow".to_owned(),
                    Box::new(SlowPlugin {
                        delay: Duration::from_millis(200),
                        started: Arc::clone(&started),
                    }),
                ),
                ("recorder".to_owned(), Box::new(plugin)),
            ],
            RedactionPolicy {
                enabled: false,
                ..RedactionPolicy::default()
            },
            Duration::from_millis(20),
        )
        .expect("runtime should start");
        let handle = runtime.handle();
        handle.dispatch_event(sample_event(SqlEventKind::Query, false));

        wait_for(|| {
            handles
                .calls
                .lock()
                .expect("lock")
                .contains(&"query".to_owned())
        })
        .await;
        assert!(started.load(Ordering::SeqCst) >= 1);
        runtime.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn plugins_are_dispatched_in_registration_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let runtime = PluginRuntime::start_with_plugins(
            vec![
                (
                    "a".to_owned(),
                    Box::new(OrderPlugin {
                        name: "a".to_owned(),
                        order: Arc::clone(&order),
                    }),
                ),
                (
                    "b".to_owned(),
                    Box::new(OrderPlugin {
                        name: "b".to_owned(),
                        order: Arc::clone(&order),
                    }),
                ),
            ],
            RedactionPolicy {
                enabled: false,
                ..RedactionPolicy::default()
            },
            Duration::from_millis(200),
        )
        .expect("runtime should start");
        runtime
            .handle()
            .dispatch_event(sample_event(SqlEventKind::Query, false));

        wait_for(|| order.lock().expect("lock").len() >= 2).await;
        runtime.shutdown().await.expect("shutdown should succeed");
        assert_eq!(*order.lock().expect("lock"), ["a", "b"]);
    }

    #[tokio::test]
    async fn shutdown_drains_queued_payloads() {
        let (plugin, handles) = RecordingPlugin::shared();
        let runtime = PluginRuntime::start_with_plugins(
            vec![("recorder".to_owned(), Box::new(plugin))],
            RedactionPolicy {
                enabled: false,
                ..RedactionPolicy::default()
            },
            Duration::from_millis(200),
        )
        .expect("runtime should start");
        let handle = runtime.handle();
        for _ in 0..5 {
            handle.dispatch_event(sample_event(SqlEventKind::Query, false));
        }
        runtime.shutdown().await.expect("shutdown should succeed");
        assert_eq!(handles.calls.lock().expect("lock").len(), 5);
    }

    async fn wait_for(mut predicate: impl FnMut() -> bool) {
        for _ in 0..100 {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for plugin side effect");
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir().join(format!("sql-lens-{label}-{nanos}"))
    }

    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_connection() -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId("conn_plugin".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            state: ConnectionState::Ready,
            connected_at: Timestamp("2026-07-10T12:00:00Z".to_owned()),
            closed_at: None,
            last_activity_at: Some(Timestamp("2026-07-10T12:00:01Z".to_owned())),
            bytes_in: 0,
            bytes_out: 0,
            query_count: 0,
        }
    }

    fn sample_event(kind: SqlEventKind, with_error: bool) -> SqlEvent {
        SqlEvent {
            id: SqlEventId("evt_plugin".to_owned()),
            timestamp: Timestamp("2026-07-10T12:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_plugin".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            kind,
            status: if with_error {
                CaptureStatus::Error
            } else {
                CaptureStatus::Ok
            },
            duration: DurationMillis(3),
            original_sql: "SELECT * FROM users WHERE password = 'secret-password'".to_owned(),
            normalized_sql: Some(
                "select * from users where password = 'secret-password'".to_owned(),
            ),
            expanded_sql: Some("SELECT * FROM users WHERE password = 'secret-password'".to_owned()),
            fingerprint: Some("select * from users where password = ?".to_owned()),
            parameters: vec![SqlParameter {
                index: 0,
                name: Some("password".to_owned()),
                value: SqlParameterValue::String("secret-password".to_owned()),
                redacted: false,
            }],
            result: None,
            error: with_error.then(|| ErrorSummary {
                code: Some("42".to_owned()),
                sql_state: None,
                message: "boom".to_owned(),
                metadata: None,
            }),
            timings: QueryTiming {
                started_at: Timestamp("2026-07-10T12:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-10T12:00:00Z".to_owned())),
                duration: DurationMillis(3),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: Vec::new(),
            },
        }
    }
}
