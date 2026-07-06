use clap::Parser;
use sql_lens_config::{ConfigLoadError, ConfigValidationError, SqlLensConfig};
use std::{error::Error, fmt, path::PathBuf, process::ExitCode};

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

    Ok(())
}

#[derive(Debug)]
enum AppError {
    ConfigLoad(ConfigLoadError),
    ConfigValidation(ConfigValidationError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigLoad(source) => write!(f, "failed to load SQL Lens config: {source}"),
            Self::ConfigValidation(source) => {
                write!(f, "failed to validate SQL Lens config: {source}")
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConfigLoad(source) => Some(source),
            Self::ConfigValidation(source) => Some(source),
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
