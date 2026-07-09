use std::{error::Error, fmt, str};

pub const MYSQL_COM_QUERY: u8 = 0x03;
pub const MYSQL_COM_PING: u8 = 0x0e;
pub const MYSQL_COM_STMT_PREPARE: u8 = 0x16;
pub const MYSQL_COM_STMT_EXECUTE: u8 = 0x17;
pub const MYSQL_COM_STMT_CLOSE: u8 = 0x19;
pub const MYSQL_COM_QUIT: u8 = 0x01;
pub const MYSQL_CLIENT_QUERY_ATTRIBUTES: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MysqlCommandKind {
    Query,
    Ping,
    StatementPrepare,
    StatementExecute,
    StatementClose,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComQuery {
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComStmtPrepare {
    pub template_sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComStmtExecute {
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComStmtClose {
    pub statement_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComPing;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlComQuit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    Ping(MysqlComPing),
    StatementPrepare(MysqlComStmtPrepare),
    StatementExecute(MysqlComStmtExecute),
    StatementClose(MysqlComStmtClose),
    Quit(MysqlComQuit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlClientCommand {
    pub kind: MysqlCommandKind,
    pub sequence_id: u8,
    pub sql: String,
}

pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlParsedClientCommand>, MysqlCommandParseError> {
    parse_client_command_with_capabilities(payload, 0)
}

pub fn parse_client_command_with_capabilities(
    payload: &[u8],
    client_capability_flags: u32,
) -> Result<Option<MysqlParsedClientCommand>, MysqlCommandParseError> {
    let Some((&command, command_body)) = payload.split_first() else {
        return Err(MysqlCommandParseError::IncompletePayload {
            field: "command",
            needed: 1,
            available: 0,
        });
    };

    match command {
        MYSQL_COM_QUIT => Ok(Some(MysqlParsedClientCommand::Quit(MysqlComQuit))),
        MYSQL_COM_QUERY => {
            let command_body = com_query_sql_payload(command_body, client_capability_flags)?;
            let sql = parse_utf8_field(command_body, "sql")?;

            Ok(Some(MysqlParsedClientCommand::Query(MysqlComQuery { sql })))
        }
        MYSQL_COM_PING => Ok(Some(MysqlParsedClientCommand::Ping(MysqlComPing))),
        MYSQL_COM_STMT_PREPARE => {
            let template_sql = parse_utf8_field(command_body, "template_sql")?;

            Ok(Some(MysqlParsedClientCommand::StatementPrepare(
                MysqlComStmtPrepare { template_sql },
            )))
        }
        MYSQL_COM_STMT_EXECUTE => Ok(Some(MysqlParsedClientCommand::StatementExecute(
            parse_com_stmt_execute(command_body)?,
        ))),
        MYSQL_COM_STMT_CLOSE => Ok(Some(MysqlParsedClientCommand::StatementClose(
            parse_com_stmt_close(command_body)?,
        ))),
        _ => Ok(None),
    }
}

fn com_query_sql_payload(
    command_body: &[u8],
    client_capability_flags: u32,
) -> Result<&[u8], MysqlCommandParseError> {
    if client_capability_flags & MYSQL_CLIENT_QUERY_ATTRIBUTES == 0 {
        return Ok(command_body);
    }

    let (parameter_count, parameter_count_len) =
        read_lenenc_integer(command_body, "query_attribute_parameter_count")?;
    let after_parameter_count = &command_body[parameter_count_len..];
    let (_, parameter_set_count_len) =
        read_lenenc_integer(after_parameter_count, "query_attribute_parameter_set_count")?;
    let sql_payload = &after_parameter_count[parameter_set_count_len..];

    if parameter_count == 0 {
        return Ok(sql_payload);
    }

    Err(MysqlCommandParseError::UnsupportedQueryAttributes { parameter_count })
}

fn parse_com_stmt_execute(bytes: &[u8]) -> Result<MysqlComStmtExecute, MysqlCommandParseError> {
    let statement_id = read_u32_le(bytes, "statement_id")?;
    let Some((&flags, iteration_count_bytes)) =
        bytes.get(4..).and_then(|bytes| bytes.split_first())
    else {
        return Err(MysqlCommandParseError::IncompletePayload {
            field: "flags",
            needed: 1,
            available: bytes.len().saturating_sub(4),
        });
    };
    let iteration_count = read_u32_le(iteration_count_bytes, "iteration_count")?;

    Ok(MysqlComStmtExecute {
        statement_id,
        flags,
        iteration_count,
        has_parameter_payload: iteration_count_bytes.len() > 4,
    })
}

fn parse_com_stmt_close(bytes: &[u8]) -> Result<MysqlComStmtClose, MysqlCommandParseError> {
    Ok(MysqlComStmtClose {
        statement_id: read_u32_le(bytes, "statement_id")?,
    })
}

fn read_u32_le(bytes: &[u8], field: &'static str) -> Result<u32, MysqlCommandParseError> {
    let Some(value) = bytes.get(..4) else {
        return Err(MysqlCommandParseError::IncompletePayload {
            field,
            needed: 4,
            available: bytes.len(),
        });
    };

    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_lenenc_integer(
    bytes: &[u8],
    field: &'static str,
) -> Result<(u64, usize), MysqlCommandParseError> {
    let Some((&first, rest)) = bytes.split_first() else {
        return Err(MysqlCommandParseError::IncompletePayload {
            field,
            needed: 1,
            available: 0,
        });
    };

    match first {
        0x00..=0xfa => Ok((u64::from(first), 1)),
        0xfc => read_fixed_lenenc_integer(rest, 2, field).map(|value| (value, 3)),
        0xfd => read_fixed_lenenc_integer(rest, 3, field).map(|value| (value, 4)),
        0xfe => read_fixed_lenenc_integer(rest, 8, field).map(|value| (value, 9)),
        marker => Err(MysqlCommandParseError::InvalidLengthEncodedInteger { field, marker }),
    }
}

fn read_fixed_lenenc_integer(
    bytes: &[u8],
    len: usize,
    field: &'static str,
) -> Result<u64, MysqlCommandParseError> {
    if bytes.len() < len {
        return Err(MysqlCommandParseError::IncompletePayload {
            field,
            needed: len,
            available: bytes.len(),
        });
    }

    let mut value = 0_u64;
    for (index, byte) in bytes[..len].iter().enumerate() {
        value |= u64::from(*byte) << (index * 8);
    }

    Ok(value)
}

fn parse_utf8_field(bytes: &[u8], field: &'static str) -> Result<String, MysqlCommandParseError> {
    str::from_utf8(bytes)
        .map_err(|_| MysqlCommandParseError::InvalidUtf8 { field })
        .map(str::to_owned)
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
    InvalidLengthEncodedInteger {
        field: &'static str,
        marker: u8,
    },
    UnsupportedQueryAttributes {
        parameter_count: u64,
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
            Self::InvalidLengthEncodedInteger { field, marker } => write!(
                f,
                "invalid length-encoded integer marker 0x{marker:02x} in MySQL command field `{field}`"
            ),
            Self::UnsupportedQueryAttributes { parameter_count } => write!(
                f,
                "unsupported MySQL COM_QUERY attributes with {parameter_count} parameters"
            ),
        }
    }
}

impl Error for MysqlCommandParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_com_quit() {
        let command = parse_client_command(&[MYSQL_COM_QUIT])
            .expect("command should parse")
            .expect("COM_QUIT should be supported");

        assert_eq!(command, MysqlParsedClientCommand::Quit(MysqlComQuit));
    }

    #[test]
    fn parses_com_query_sql_text() {
        let mut payload = vec![MYSQL_COM_QUERY];
        payload.extend_from_slice(b"select 1");

        let command = parse_client_command(&payload)
            .expect("command should parse")
            .expect("COM_QUERY should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::Query(MysqlComQuery {
                sql: "select 1".to_owned(),
            })
        );
    }

    #[test]
    fn parses_com_query_sql_text_after_empty_query_attributes() {
        let payload = [
            MYSQL_COM_QUERY,
            0x00, // parameter_count
            0x01, // parameter_set_count
            b's',
            b'e',
            b'l',
            b'e',
            b'c',
            b't',
            b' ',
            b'1',
        ];

        let command =
            parse_client_command_with_capabilities(&payload, MYSQL_CLIENT_QUERY_ATTRIBUTES)
                .expect("command should parse")
                .expect("COM_QUERY should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::Query(MysqlComQuery {
                sql: "select 1".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_com_query_attributes_with_parameter_payload() {
        let payload = [MYSQL_COM_QUERY, 0x01, 0x01, b'x'];

        let error = parse_client_command_with_capabilities(&payload, MYSQL_CLIENT_QUERY_ATTRIBUTES)
            .expect_err("query attributes should be unsupported until parsed");

        assert_eq!(
            error,
            MysqlCommandParseError::UnsupportedQueryAttributes { parameter_count: 1 }
        );
    }

    #[test]
    fn parses_empty_com_query_sql_text() {
        let command = parse_client_command(&[MYSQL_COM_QUERY])
            .expect("command should parse")
            .expect("COM_QUERY should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::Query(MysqlComQuery { sql: String::new() })
        );
    }

    #[test]
    fn parses_com_ping() {
        let command = parse_client_command(&[MYSQL_COM_PING])
            .expect("command should parse")
            .expect("COM_PING should be supported");

        assert_eq!(command, MysqlParsedClientCommand::Ping(MysqlComPing));
    }

    #[test]
    fn parses_com_stmt_prepare_template_sql() {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE];
        payload.extend_from_slice(b"select * from users where id = ?");

        let command = parse_client_command(&payload)
            .expect("command should parse")
            .expect("COM_STMT_PREPARE should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::StatementPrepare(MysqlComStmtPrepare {
                template_sql: "select * from users where id = ?".to_owned(),
            })
        );
    }

    #[test]
    fn parses_empty_com_stmt_prepare_template_sql() {
        let command = parse_client_command(&[MYSQL_COM_STMT_PREPARE])
            .expect("command should parse")
            .expect("COM_STMT_PREPARE should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::StatementPrepare(MysqlComStmtPrepare {
                template_sql: String::new(),
            })
        );
    }

    #[test]
    fn parses_com_stmt_execute_envelope() {
        let command = parse_client_command(&[
            MYSQL_COM_STMT_EXECUTE,
            0x44,
            0x33,
            0x22,
            0x11,
            0x02,
            0x04,
            0x03,
            0x02,
            0x01,
        ])
        .expect("command should parse")
        .expect("COM_STMT_EXECUTE should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::StatementExecute(MysqlComStmtExecute {
                statement_id: 0x1122_3344,
                flags: 0x02,
                iteration_count: 0x0102_0304,
                has_parameter_payload: false,
            })
        );
    }

    #[test]
    fn parses_com_stmt_execute_with_parameter_payload_bytes() {
        let command = parse_client_command(&[
            MYSQL_COM_STMT_EXECUTE,
            0x44,
            0x33,
            0x22,
            0x11,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x01,
        ])
        .expect("command should parse")
        .expect("COM_STMT_EXECUTE should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::StatementExecute(MysqlComStmtExecute {
                statement_id: 0x1122_3344,
                flags: 0x00,
                iteration_count: 1,
                has_parameter_payload: true,
            })
        );
    }

    #[test]
    fn parses_com_stmt_close_statement_id() {
        let command = parse_client_command(&[MYSQL_COM_STMT_CLOSE, 0x44, 0x33, 0x22, 0x11])
            .expect("command should parse")
            .expect("COM_STMT_CLOSE should be supported");

        assert_eq!(
            command,
            MysqlParsedClientCommand::StatementClose(MysqlComStmtClose {
                statement_id: 0x1122_3344,
            })
        );
    }

    #[test]
    fn returns_none_for_unsupported_command() {
        let command =
            parse_client_command(&[0x7f, b'x']).expect("unsupported command should be non-fatal");

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
    fn rejects_com_stmt_execute_missing_statement_id() {
        let error = parse_client_command(&[MYSQL_COM_STMT_EXECUTE, 0x01])
            .expect_err("statement ID should be incomplete");

        assert_eq!(
            error,
            MysqlCommandParseError::IncompletePayload {
                field: "statement_id",
                needed: 4,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_com_stmt_execute_missing_flags() {
        let error = parse_client_command(&[MYSQL_COM_STMT_EXECUTE, 0x44, 0x33, 0x22, 0x11])
            .expect_err("flags should be missing");

        assert_eq!(
            error,
            MysqlCommandParseError::IncompletePayload {
                field: "flags",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_com_stmt_close_missing_statement_id() {
        let error = parse_client_command(&[MYSQL_COM_STMT_CLOSE, 0x44, 0x33])
            .expect_err("statement ID should be incomplete");

        assert_eq!(
            error,
            MysqlCommandParseError::IncompletePayload {
                field: "statement_id",
                needed: 4,
                available: 2,
            }
        );
    }

    #[test]
    fn rejects_com_stmt_execute_missing_iteration_count() {
        let error = parse_client_command(&[
            MYSQL_COM_STMT_EXECUTE,
            0x44,
            0x33,
            0x22,
            0x11,
            0x00,
            0x01,
            0x00,
        ])
        .expect_err("iteration count should be incomplete");

        assert_eq!(
            error,
            MysqlCommandParseError::IncompletePayload {
                field: "iteration_count",
                needed: 4,
                available: 2,
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
    fn rejects_invalid_utf8_prepare_template_sql() {
        let error = parse_client_command(&[MYSQL_COM_STMT_PREPARE, 0xff])
            .expect_err("template SQL should be invalid UTF-8");

        assert_eq!(
            error,
            MysqlCommandParseError::InvalidUtf8 {
                field: "template_sql",
            }
        );
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
