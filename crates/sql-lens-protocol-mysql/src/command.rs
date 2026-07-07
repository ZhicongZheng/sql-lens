use std::{error::Error, fmt, str};

pub const MYSQL_COM_QUERY: u8 = 0x03;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MysqlCommandKind {
    Query,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComQuery {
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlClientCommand {
    pub kind: MysqlCommandKind,
    pub sequence_id: u8,
    pub sql: String,
}

pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlComQuery>, MysqlCommandParseError> {
    let Some((&command, sql_bytes)) = payload.split_first() else {
        return Err(MysqlCommandParseError::IncompletePayload {
            field: "command",
            needed: 1,
            available: 0,
        });
    };

    match command {
        MYSQL_COM_QUERY => {
            let sql = str::from_utf8(sql_bytes)
                .map_err(|_| MysqlCommandParseError::InvalidUtf8 { field: "sql" })?
                .to_owned();

            Ok(Some(MysqlComQuery { sql }))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlCommandParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    InvalidUtf8 {
        field: &'static str,
    },
}

impl fmt::Display for MysqlCommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL command field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::InvalidUtf8 { field } => {
                write!(f, "invalid UTF-8 in MySQL command field `{field}`")
            }
        }
    }
}

impl Error for MysqlCommandParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_com_query_sql_text() {
        let mut payload = vec![MYSQL_COM_QUERY];
        payload.extend_from_slice(b"select 1");

        let command = parse_client_command(&payload)
            .expect("command should parse")
            .expect("COM_QUERY should be supported");

        assert_eq!(command.sql, "select 1");
    }

    #[test]
    fn parses_empty_com_query_sql_text() {
        let command = parse_client_command(&[MYSQL_COM_QUERY])
            .expect("command should parse")
            .expect("COM_QUERY should be supported");

        assert_eq!(command.sql, "");
    }

    #[test]
    fn returns_none_for_unsupported_command() {
        let command =
            parse_client_command(&[0x01, b'x']).expect("unsupported command should be non-fatal");

        assert_eq!(command, None);
    }

    #[test]
    fn rejects_empty_payload() {
        let error = parse_client_command(&[]).expect_err("command byte should be missing");

        assert_eq!(
            error,
            MysqlCommandParseError::IncompletePayload {
                field: "command",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_invalid_utf8_sql_text() {
        let error = parse_client_command(&[MYSQL_COM_QUERY, 0xff])
            .expect_err("SQL should be invalid UTF-8");

        assert_eq!(error, MysqlCommandParseError::InvalidUtf8 { field: "sql" });
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlCommandParseError::IncompletePayload {
                field: "command",
                needed: 1,
                available: 0,
            }
            .to_string(),
            "incomplete MySQL command field `command`: needed 1 bytes, available 0 bytes"
        );
        assert_eq!(
            MysqlCommandParseError::InvalidUtf8 { field: "sql" }.to_string(),
            "invalid UTF-8 in MySQL command field `sql`"
        );
    }
}
