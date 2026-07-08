use serde::{Deserialize, Serialize};

use crate::{SqlEvent, SqlParameterValue};

pub const DEFAULT_REDACTION_MASK: &str = "***";
pub const DEFAULT_REDACTION_PARAMETER_NAMES: &[&str] = &[
    "password",
    "passwd",
    "token",
    "secret",
    "api_key",
    "access_key",
    "refresh_token",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionPolicy {
    pub enabled: bool,
    pub mask: String,
    pub parameter_names: Vec<String>,
    pub sql_patterns: Vec<String>,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            mask: DEFAULT_REDACTION_MASK.to_owned(),
            parameter_names: DEFAULT_REDACTION_PARAMETER_NAMES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            sql_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SqlTextReplacement {
    pattern: String,
    replacement: String,
}

pub fn redact_sql_event(mut event: SqlEvent, policy: &RedactionPolicy) -> SqlEvent {
    if !policy.enabled {
        return event;
    }

    let mut replacements = sql_pattern_replacements(policy);

    for parameter in &mut event.parameters {
        if parameter.redacted || parameter_name_matches(parameter.name.as_deref(), policy) {
            replacements.extend(parameter_value_replacements(&parameter.value, &policy.mask));
            parameter.redacted = true;
            parameter.value = SqlParameterValue::String(policy.mask.clone());
        }
    }

    apply_replacements(&mut event.original_sql, &replacements);
    if let Some(normalized_sql) = &mut event.normalized_sql {
        apply_replacements(normalized_sql, &replacements);
    }
    if let Some(expanded_sql) = &mut event.expanded_sql {
        apply_replacements(expanded_sql, &replacements);
    }

    event
}

fn parameter_name_matches(name: Option<&str>, policy: &RedactionPolicy) -> bool {
    let Some(name) = name else {
        return false;
    };

    policy
        .parameter_names
        .iter()
        .filter(|candidate| !candidate.is_empty())
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn sql_pattern_replacements(policy: &RedactionPolicy) -> Vec<SqlTextReplacement> {
    policy
        .sql_patterns
        .iter()
        .filter(|pattern| !pattern.is_empty())
        .map(|pattern| SqlTextReplacement {
            pattern: pattern.clone(),
            replacement: policy.mask.clone(),
        })
        .collect()
}

fn parameter_value_replacements(value: &SqlParameterValue, mask: &str) -> Vec<SqlTextReplacement> {
    match value {
        SqlParameterValue::Null => Vec::new(),
        SqlParameterValue::Integer(value) => scalar_replacement(value.to_string(), mask),
        SqlParameterValue::Unsigned(value) => scalar_replacement(value.to_string(), mask),
        SqlParameterValue::Float(value) => scalar_replacement(value.to_string(), mask),
        SqlParameterValue::Boolean(value) => {
            scalar_replacement(if *value { "TRUE" } else { "FALSE" }.to_owned(), mask)
        }
        SqlParameterValue::String(value)
        | SqlParameterValue::Date(value)
        | SqlParameterValue::Time(value)
        | SqlParameterValue::Timestamp(value)
        | SqlParameterValue::Json(value)
        | SqlParameterValue::BinarySummary(value)
        | SqlParameterValue::Unsupported(value) => string_replacements(value, mask),
    }
}

fn scalar_replacement(value: String, mask: &str) -> Vec<SqlTextReplacement> {
    if value.is_empty() {
        return Vec::new();
    }

    vec![SqlTextReplacement {
        pattern: value,
        replacement: mask.to_owned(),
    }]
}

fn string_replacements(value: &str, mask: &str) -> Vec<SqlTextReplacement> {
    if value.is_empty() {
        return Vec::new();
    }

    vec![
        SqlTextReplacement {
            pattern: value.to_owned(),
            replacement: mask.to_owned(),
        },
        SqlTextReplacement {
            pattern: quote_sql_display_literal(value),
            replacement: quote_sql_display_literal(mask),
        },
    ]
}

fn apply_replacements(sql: &mut String, replacements: &[SqlTextReplacement]) {
    for replacement in replacements {
        if sql.contains(&replacement.pattern) {
            *sql = sql.replace(&replacement.pattern, &replacement.replacement);
        }
    }
}

fn quote_sql_display_literal(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push('\'');
            quoted.push('\'');
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod tests {
    use crate::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ProtocolMetadata, ProtocolName,
        QueryTiming, RedactionPolicy, SqlEvent, SqlEventId, SqlEventKind, SqlParameter,
        SqlParameterValue, Timestamp, redact_sql_event,
    };

    fn test_event() -> SqlEvent {
        SqlEvent {
            id: SqlEventId("evt_1".to_owned()),
            timestamp: Timestamp("2026-07-08T09:00:00Z".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_1".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            kind: SqlEventKind::StatementExecute,
            status: CaptureStatus::Ok,
            duration: DurationMillis(3),
            original_sql: "SELECT * FROM users WHERE password = ?".to_owned(),
            normalized_sql: Some("select * from users where password = ?".to_owned()),
            expanded_sql: Some("SELECT * FROM users WHERE password = 's3cr3t'".to_owned()),
            fingerprint: Some("select * from users where password = ?".to_owned()),
            parameters: vec![SqlParameter {
                index: 0,
                name: Some("password".to_owned()),
                value: SqlParameterValue::String("s3cr3t".to_owned()),
                redacted: false,
            }],
            result: None,
            error: None,
            timings: QueryTiming {
                started_at: Timestamp("2026-07-08T09:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-08T09:00:00Z".to_owned())),
                duration: DurationMillis(3),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: Vec::new(),
            },
        }
    }

    #[test]
    fn disabled_policy_leaves_event_unchanged() {
        let event = test_event();
        let policy = RedactionPolicy {
            enabled: false,
            ..RedactionPolicy::default()
        };

        assert_eq!(redact_sql_event(event.clone(), &policy), event);
    }

    #[test]
    fn sensitive_parameter_names_match_case_insensitively() {
        let mut event = test_event();
        event.parameters[0].name = Some("PassWord".to_owned());

        let redacted = redact_sql_event(event, &RedactionPolicy::default());

        assert!(redacted.parameters[0].redacted);
        assert_eq!(
            redacted.parameters[0].value,
            SqlParameterValue::String("***".to_owned())
        );
        assert_eq!(
            redacted.expanded_sql.as_deref(),
            Some("SELECT * FROM users WHERE password = '***'")
        );
    }

    #[test]
    fn already_redacted_parameters_are_masked_again() {
        let mut event = test_event();
        event.parameters[0].name = Some("id".to_owned());
        event.parameters[0].redacted = true;
        event.parameters[0].value = SqlParameterValue::String("raw-token".to_owned());
        event.expanded_sql = Some("SELECT * FROM sessions WHERE id = 'raw-token'".to_owned());

        let redacted = redact_sql_event(event, &RedactionPolicy::default());

        assert!(redacted.parameters[0].redacted);
        assert_eq!(
            redacted.parameters[0].value,
            SqlParameterValue::String("***".to_owned())
        );
        assert_eq!(
            redacted.expanded_sql.as_deref(),
            Some("SELECT * FROM sessions WHERE id = '***'")
        );
    }

    #[test]
    fn sql_patterns_apply_to_all_sql_text_fields() {
        let mut event = test_event();
        event.original_sql = "SELECT card_number FROM payments".to_owned();
        event.normalized_sql = Some("select card_number from payments".to_owned());
        event.expanded_sql = Some("SELECT card_number FROM payments".to_owned());
        event.parameters.clear();
        let policy = RedactionPolicy {
            sql_patterns: vec!["card_number".to_owned()],
            ..RedactionPolicy::default()
        };

        let redacted = redact_sql_event(event, &policy);

        assert_eq!(redacted.original_sql, "SELECT *** FROM payments");
        assert_eq!(
            redacted.normalized_sql.as_deref(),
            Some("select *** from payments")
        );
        assert_eq!(
            redacted.expanded_sql.as_deref(),
            Some("SELECT *** FROM payments")
        );
    }

    #[test]
    fn expanded_sql_value_masking_handles_quoted_display_literals() {
        let mut event = test_event();
        event.parameters[0].value = SqlParameterValue::String("O'Reilly".to_owned());
        event.expanded_sql = Some("SELECT * FROM users WHERE password = 'O''Reilly'".to_owned());

        let redacted = redact_sql_event(event, &RedactionPolicy::default());

        assert_eq!(
            redacted.expanded_sql.as_deref(),
            Some("SELECT * FROM users WHERE password = '***'")
        );
    }

    #[test]
    fn empty_patterns_and_empty_sensitive_values_are_ignored() {
        let mut event = test_event();
        event.parameters[0].value = SqlParameterValue::String(String::new());
        event.expanded_sql = Some("SELECT '' AS password".to_owned());
        let policy = RedactionPolicy {
            sql_patterns: vec![String::new()],
            ..RedactionPolicy::default()
        };

        let redacted = redact_sql_event(event, &policy);

        assert_eq!(
            redacted.expanded_sql.as_deref(),
            Some("SELECT '' AS password")
        );
        assert_eq!(
            redacted.parameters[0].value,
            SqlParameterValue::String("***".to_owned())
        );
    }
}
