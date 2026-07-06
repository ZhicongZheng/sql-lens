use clap::Parser;
use sql_lens_config::{
    ConfigLoadError, ConfigValidationError, LoggingConfig, LoggingFormat, LoggingLevel,
    SqlLensConfig,
};
use std::{error::Error, fmt, path::PathBuf, process::ExitCode};
use tracing_subscriber::filter::LevelFilter;

const STARTUP_CHECK_LOG_MESSAGE: &str = "SQL Lens startup checks completed";

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

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    let config = SqlLensConfig::from_path(&cli.config)?;
    config.validate()?;
    init_logging(&config.logging)?;

    tracing::info!("{STARTUP_CHECK_LOG_MESSAGE}");

    Ok(())
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
    ConfigValidation(ConfigValidationError),
    LoggingInit(Box<dyn Error + Send + Sync + 'static>),
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
            Self::ConfigValidation(source) => {
                write!(f, "failed to validate SQL Lens config: {source}")
            }
            Self::LoggingInit(source) => {
                write!(f, "failed to initialize SQL Lens logging: {source}")
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConfigLoad(source) => Some(source),
            Self::ConfigValidation(source) => Some(source),
            Self::LoggingInit(source) => Some(source.as_ref()),
        }
    }
}

impl From<ConfigLoadError> for AppError {
    fn from(source: ConfigLoadError) -> Self {
        Self::ConfigLoad(source)
    }
}

impl From<ConfigValidationError> for AppError {
    fn from(source: ConfigValidationError) -> Self {
        Self::ConfigValidation(source)
    }
}
