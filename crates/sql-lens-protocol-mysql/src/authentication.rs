use std::{error::Error, fmt, str};

const OK_PACKET_HEADER: u8 = 0x00;
const ERR_PACKET_HEADER: u8 = 0xff;
const SQL_STATE_MARKER: u8 = b'#';
const SQL_STATE_LEN: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MysqlAuthenticationStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlAuthenticationResult {
    pub status: MysqlAuthenticationStatus,
    pub error_code: Option<u16>,
    pub sql_state: Option<String>,
    pub message: Option<String>,
}

pub fn parse_authentication_result(
    payload: &[u8],
) -> Result<Option<MysqlAuthenticationResult>, MysqlAuthenticationResultParseError> {
    let Some((&header, rest)) = payload.split_first() else {
        return Err(MysqlAuthenticationResultParseError::IncompletePayload {
            field: "header",
            needed: 1,
            available: 0,
        });
    };

    match header {
        OK_PACKET_HEADER => Ok(Some(MysqlAuthenticationResult {
            status: MysqlAuthenticationStatus::Succeeded,
            error_code: None,
            sql_state: None,
            message: None,
        })),
        ERR_PACKET_HEADER => parse_error_packet(rest).map(Some),
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlAuthenticationResultParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    InvalidUtf8 {
        field: &'static str,
    },
}

impl fmt::Display for MysqlAuthenticationResultParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL authentication result field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::InvalidUtf8 { field } => {
                write!(
                    f,
                    "invalid UTF-8 in MySQL authentication result field `{field}`"
                )
            }
        }
    }
}

impl Error for MysqlAuthenticationResultParseError {}

fn parse_error_packet(
    payload: &[u8],
) -> Result<MysqlAuthenticationResult, MysqlAuthenticationResultParseError> {
    let mut offset = 0;
    let error_code = read_optional_error_code(payload, &mut offset)?;
    let sql_state = read_optional_sql_state(payload, &mut offset)?;
    let message = read_optional_message(payload, offset)?;

    Ok(MysqlAuthenticationResult {
        status: MysqlAuthenticationStatus::Failed,
        error_code,
        sql_state,
        message,
    })
}

fn read_optional_error_code(
    payload: &[u8],
    offset: &mut usize,
) -> Result<Option<u16>, MysqlAuthenticationResultParseError> {
    let available = payload.len().saturating_sub(*offset);

    if available == 0 {
        return Ok(None);
    }

    if available < 2 {
        return Err(MysqlAuthenticationResultParseError::IncompletePayload {
            field: "error_code",
            needed: 2,
            available,
        });
    }

    let code = u16::from_le_bytes([payload[*offset], payload[*offset + 1]]);
    *offset += 2;

    Ok(Some(code))
}

fn read_optional_sql_state(
    payload: &[u8],
    offset: &mut usize,
) -> Result<Option<String>, MysqlAuthenticationResultParseError> {
    if payload.get(*offset) != Some(&SQL_STATE_MARKER) {
        return Ok(None);
    }

    let available = payload.len().saturating_sub(*offset + 1);
    if available < SQL_STATE_LEN {
        return Err(MysqlAuthenticationResultParseError::IncompletePayload {
            field: "sql_state",
            needed: SQL_STATE_LEN,
            available,
        });
    }

    *offset += 1;
    let state = read_utf8(&payload[*offset..*offset + SQL_STATE_LEN], "sql_state")?.to_owned();
    *offset += SQL_STATE_LEN;

    Ok(Some(state))
}

fn read_optional_message(
    payload: &[u8],
    offset: usize,
) -> Result<Option<String>, MysqlAuthenticationResultParseError> {
    let message = &payload[offset..];

    if message.is_empty() {
        return Ok(None);
    }

    Ok(Some(read_utf8(message, "message")?.to_owned()))
}

fn read_utf8<'a>(
    value: &'a [u8],
    field: &'static str,
) -> Result<&'a str, MysqlAuthenticationResultParseError> {
    str::from_utf8(value).map_err(|_| MysqlAuthenticationResultParseError::InvalidUtf8 { field })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ok_packet_as_success() {
        let result = parse_authentication_result(&[OK_PACKET_HEADER])
            .expect("auth result should parse")
            .expect("OK packet should produce result");

        assert_eq!(result.status, MysqlAuthenticationStatus::Succeeded);
        assert_eq!(result.error_code, None);
        assert_eq!(result.sql_state, None);
        assert_eq!(result.message, None);
    }

    #[test]
    fn parses_err_packet_with_safe_metadata() {
        let mut payload = vec![ERR_PACKET_HEADER];
        payload.extend_from_slice(&1045u16.to_le_bytes());
        payload.push(SQL_STATE_MARKER);
        payload.extend_from_slice(b"28000");
        payload.extend_from_slice(b"Access denied");

        let result = parse_authentication_result(&payload)
            .expect("auth result should parse")
            .expect("ERR packet should produce result");

        assert_eq!(result.status, MysqlAuthenticationStatus::Failed);
        assert_eq!(result.error_code, Some(1045));
        assert_eq!(result.sql_state, Some("28000".to_owned()));
        assert_eq!(result.message, Some("Access denied".to_owned()));
    }

    #[test]
    fn parses_err_packet_without_optional_metadata() {
        let result = parse_authentication_result(&[ERR_PACKET_HEADER])
            .expect("auth result should parse")
            .expect("ERR packet should produce result");

        assert_eq!(result.status, MysqlAuthenticationStatus::Failed);
        assert_eq!(result.error_code, None);
        assert_eq!(result.sql_state, None);
        assert_eq!(result.message, None);
    }

    #[test]
    fn returns_none_for_unsupported_auth_continuation_packet() {
        let result = parse_authentication_result(&[0xfe, b'a', b'u', b't', b'h'])
            .expect("unsupported auth continuation should be non-fatal");

        assert_eq!(result, None);
    }

    #[test]
    fn rejects_empty_payload() {
        let error = parse_authentication_result(&[]).expect_err("payload should be incomplete");

        assert_eq!(
            error,
            MysqlAuthenticationResultParseError::IncompletePayload {
                field: "header",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_incomplete_error_code() {
        let error = parse_authentication_result(&[ERR_PACKET_HEADER, 0x15])
            .expect_err("error code should be incomplete");

        assert_eq!(
            error,
            MysqlAuthenticationResultParseError::IncompletePayload {
                field: "error_code",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_invalid_utf8_message() {
        let mut payload = vec![ERR_PACKET_HEADER];
        payload.extend_from_slice(&1045u16.to_le_bytes());
        payload.push(SQL_STATE_MARKER);
        payload.extend_from_slice(b"28000");
        payload.push(0xff);

        let error =
            parse_authentication_result(&payload).expect_err("message should be invalid UTF-8");

        assert_eq!(
            error,
            MysqlAuthenticationResultParseError::InvalidUtf8 { field: "message" }
        );
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlAuthenticationResultParseError::IncompletePayload {
                field: "header",
                needed: 1,
                available: 0,
            }
            .to_string(),
            "incomplete MySQL authentication result field `header`: needed 1 bytes, available 0 bytes"
        );
        assert_eq!(
            MysqlAuthenticationResultParseError::InvalidUtf8 { field: "message" }.to_string(),
            "invalid UTF-8 in MySQL authentication result field `message`"
        );
    }
}
