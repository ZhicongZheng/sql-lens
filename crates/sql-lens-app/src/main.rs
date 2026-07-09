use clap::Parser;
use sql_lens_app::{MinimalMysqlRuntimeError, start_runtime_from_config};
use sql_lens_config::{
    ConfigLoadError, ConfigOverrideError, ConfigValidationError, LoggingConfig, LoggingFormat,
    LoggingLevel, SqlLensConfig,
};
use std::{error::Error, fmt, path::PathBuf, process::ExitCode};
use tracing_subscriber::filter::LevelFilter;

const STARTUP_CHECK_LOG_MESSAGE: &str = "SQL Lens startup checks completed";
const SHUTDOWN_SIGNAL_LOG_MESSAGE: &str = "SQL Lens shutdown signal received";

#[derive(Debug, Parser)]
#[command(
    name = "sql-lens",
    version,
    about = "Developer-first SQL debug proxy.",
    long_about = None
)]
struct Cli {
    #[arg(long, value_name = "FILE", default_value = "sql-lens.toml")]
    config: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let mut config = SqlLensConfig::from_path(&cli.config)?;
    config.apply_env_overrides()?;
    config.validate()?;
    init_logging(&config.logging)?;

    tracing::info!("{STARTUP_CHECK_LOG_MESSAGE}");
    let runtime = start_runtime_from_config(&config).await?;
    wait_for_shutdown_signal().await?;
    tracing::info!("{SHUTDOWN_SIGNAL_LOG_MESSAGE}");
    runtime.shutdown().await?;

    Ok(())
}

async fn wait_for_shutdown_signal() -> Result<(), AppError> {
    tokio::signal::ctrl_c()
        .await
        .map_err(AppError::ShutdownSignal)
}

fn init_logging(config: &LoggingConfig) -> Result<(), AppError> {
    let max_level = level_filter(config.level);

    match config.format {
        LoggingFormat::Json => tracing_subscriber::fmt()
            .with_max_level(max_level)
            .with_writer(std::io::stderr)
            .json()
            .try_init(),
        LoggingFormat::Pretty => tracing_subscriber::fmt()
            .with_max_level(max_level)
            .with_writer(std::io::stderr)
            .pretty()
            .with_ansi(false)
            .try_init(),
    }
    .map_err(AppError::logging_init)
}

fn level_filter(level: LoggingLevel) -> LevelFilter {
    match level {
        LoggingLevel::Trace => LevelFilter::TRACE,
        LoggingLevel::Debug => LevelFilter::DEBUG,
        LoggingLevel::Info => LevelFilter::INFO,
        LoggingLevel::Warn => LevelFilter::WARN,
        LoggingLevel::Error => LevelFilter::ERROR,
    }
}

#[derive(Debug)]
enum AppError {
    ConfigLoad(ConfigLoadError),
    ConfigOverride(ConfigOverrideError),
    ConfigValidation(ConfigValidationError),
    LoggingInit(Box<dyn Error + Send + Sync + 'static>),
    Runtime(MinimalMysqlRuntimeError),
    ShutdownSignal(std::io::Error),
}

impl AppError {
    fn logging_init(source: Box<dyn Error + Send + Sync + 'static>) -> Self {
        Self::LoggingInit(source)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigLoad(source) => write!(f, "failed to load SQL Lens config: {source}"),
            Self::ConfigOverride(source) => {
                write!(f, "failed to apply SQL Lens config overrides: {source}")
            }
            Self::ConfigValidation(source) => {
                write!(f, "failed to validate SQL Lens config: {source}")
            }
            Self::LoggingInit(source) => {
                write!(f, "failed to initialize SQL Lens logging: {source}")
            }
            Self::Runtime(source) => write!(f, "failed to start SQL Lens runtime: {source}"),
            Self::ShutdownSignal(source) => {
                write!(f, "failed to listen for shutdown signal: {source}")
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConfigLoad(source) => Some(source),
            Self::ConfigOverride(source) => Some(source),
            Self::ConfigValidation(source) => Some(source),
            Self::LoggingInit(source) => Some(source.as_ref()),
            Self::Runtime(source) => Some(source),
            Self::ShutdownSignal(source) => Some(source),
        }
    }
}

impl From<ConfigLoadError> for AppError {
    fn from(source: ConfigLoadError) -> Self {
        Self::ConfigLoad(source)
    }
}

impl From<ConfigOverrideError> for AppError {
    fn from(source: ConfigOverrideError) -> Self {
        Self::ConfigOverride(source)
    }
}

impl From<ConfigValidationError> for AppError {
    fn from(source: ConfigValidationError) -> Self {
        Self::ConfigValidation(source)
    }
}

impl From<MinimalMysqlRuntimeError> for AppError {
    fn from(source: MinimalMysqlRuntimeError) -> Self {
        Self::Runtime(source)
    }
}
