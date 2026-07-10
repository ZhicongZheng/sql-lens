use std::collections::HashSet;

use crate::{ConfigValidationError, ConfigValidationViolation, Protocol, SqlLensConfig};

impl SqlLensConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        let mut violations = Vec::new();

        if self.capture.capacity == 0 {
            violations.push(ConfigValidationViolation::InvalidCaptureCapacity);
        }

        if self.targets.is_empty() {
            validate_legacy_target(self, &mut violations);
        } else {
            validate_configured_targets(self, &mut violations);
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(ConfigValidationError { violations })
        }
    }
}

fn validate_legacy_target(config: &SqlLensConfig, violations: &mut Vec<ConfigValidationViolation>) {
    if config.proxy.listen.trim().is_empty() {
        violations.push(ConfigValidationViolation::MissingProxyListen);
    }

    if config.backend.address.trim().is_empty() {
        violations.push(ConfigValidationViolation::MissingBackendAddress);
    }

    if config.proxy.protocol != Protocol::MySql {
        violations.push(ConfigValidationViolation::UnsupportedProtocol {
            protocol: config.proxy.protocol,
        });
    }
}

fn validate_configured_targets(
    config: &SqlLensConfig,
    violations: &mut Vec<ConfigValidationViolation>,
) {
    let mut names = HashSet::new();
    let mut listens = HashSet::new();

    for (index, target) in config.targets.iter().enumerate() {
        let target_name = target.name.trim();
        let display_name = if target_name.is_empty() {
            format!("#{index}")
        } else {
            target_name.to_owned()
        };

        if target_name.is_empty() {
            violations.push(ConfigValidationViolation::MissingTargetName { index });
        } else if !names.insert(target_name.to_owned()) {
            violations.push(ConfigValidationViolation::DuplicateTargetName {
                name: target_name.to_owned(),
            });
        }

        let listen = target.listen.trim();
        if listen.is_empty() {
            violations.push(ConfigValidationViolation::MissingTargetListen {
                target_name: display_name.clone(),
            });
        } else if !listens.insert(listen.to_owned()) {
            violations.push(ConfigValidationViolation::DuplicateTargetListen {
                listen: listen.to_owned(),
            });
        }

        if target.backend_address.trim().is_empty() {
            violations.push(ConfigValidationViolation::MissingTargetBackendAddress {
                target_name: display_name.clone(),
            });
        }

        if target.protocol != Protocol::MySql {
            violations.push(ConfigValidationViolation::UnsupportedTargetProtocol {
                target_name: display_name,
                protocol: target.protocol,
            });
        }
    }
}
