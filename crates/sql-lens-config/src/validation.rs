use crate::{ConfigValidationError, ConfigValidationViolation, Protocol, SqlLensConfig};

impl SqlLensConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        let mut violations = Vec::new();

        if self.proxy.listen.trim().is_empty() {
            violations.push(ConfigValidationViolation::MissingProxyListen);
        }

        if self.backend.address.trim().is_empty() {
            violations.push(ConfigValidationViolation::MissingBackendAddress);
        }

        if self.proxy.protocol != Protocol::MySql {
            violations.push(ConfigValidationViolation::UnsupportedProtocol {
                protocol: self.proxy.protocol,
            });
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(ConfigValidationError { violations })
        }
    }
}
