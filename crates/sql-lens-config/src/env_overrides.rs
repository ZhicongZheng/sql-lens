use crate::{ConfigOverrideError, LoggingLevel, SqlLensConfig};

pub const SQL_LENS_PROXY_LISTEN: &str = "SQL_LENS_PROXY_LISTEN";
pub const SQL_LENS_BACKEND_ADDRESS: &str = "SQL_LENS_BACKEND_ADDRESS";
pub const SQL_LENS_LOGGING_LEVEL: &str = "SQL_LENS_LOGGING_LEVEL";

impl SqlLensConfig {
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigOverrideError> {
        self.apply_env_overrides_from(std::env::vars())
    }

    pub fn apply_env_overrides_from<I, K, V>(
        &mut self,
        variables: I,
    ) -> Result<(), ConfigOverrideError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        for (key, value) in variables {
            let key = key.as_ref();
            let value = value.as_ref();

            match key {
                SQL_LENS_PROXY_LISTEN => {
                    self.proxy.listen = value.to_owned();
                }
                SQL_LENS_BACKEND_ADDRESS => {
                    self.backend.address = value.to_owned();
                }
                SQL_LENS_LOGGING_LEVEL => {
                    self.logging.level = parse_logging_level(key, value)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn parse_logging_level(variable: &str, value: &str) -> Result<LoggingLevel, ConfigOverrideError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "trace" => Ok(LoggingLevel::Trace),
        "debug" => Ok(LoggingLevel::Debug),
        "info" => Ok(LoggingLevel::Info),
        "warn" => Ok(LoggingLevel::Warn),
        "error" => Ok(LoggingLevel::Error),
        _ => Err(ConfigOverrideError {
            variable: variable.to_owned(),
            value: value.to_owned(),
            expected: "one of trace, debug, info, warn, error",
        }),
    }
}
