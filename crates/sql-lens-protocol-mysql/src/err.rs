use std::{error::Error, fmt};

const MYSQL_ERR_PACKET_HEADER: u8 = 0xff;
const SQL_STATE_MARKER: u8 = b'#';
const SQL_STATE_LEN: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlErrPacketSummary {
    pub error_code: u16,
    pub sql_state: Option<String>,
    pub message: String,
}

pub fn parse_err_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlErrPacketSummary>, MysqlErrPacketParseError> {
    let Some((&header, payload)) = payload.split_first() else {
        return Err(MysqlErrPacketParseError::IncompletePayload {
            field: "header",
            needed: 1,
            available: 0,
        });
    };

    if header != MYSQL_ERR_PACKET_HEADER {
        return Ok(None);
    }

    if payload.len() < 2 {
        return Err(MysqlErrPacketParseError::IncompletePayload {
            field: "error_code",
            needed: 2,
            available: payload.len(),
        });
    }

    let error_code = u16::from_le_bytes([payload[0], payload[1]]);
    let mut offset = 2;
    let sql_state = read_optional_sql_state(payload, &mut offset)?;
    let message = sanitize_error_message(&payload[offset..]);

    Ok(Some(MysqlErrPacketSummary {
        error_code,
        sql_state,
        message,
    }))
}

fn read_optional_sql_state(
    payload: &[u8],
    offset: &mut usize,
) -> Result<Option<String>, MysqlErrPacketParseError> {
    if payload.get(*offset) != Some(&SQL_STATE_MARKER) {
        return Ok(None);
    }

    let available = payload.len().saturating_sub(*offset + 1);
    if available < SQL_STATE_LEN {
        return Err(MysqlErrPacketParseError::IncompletePayload {
            field: "sql_state",
            needed: SQL_STATE_LEN,
            available,
        });
    }

    *offset += 1;
    let sql_state =
        String::from_utf8_lossy(&payload[*offset..*offset + SQL_STATE_LEN]).into_owned();
    *offset += SQL_STATE_LEN;

    Ok(Some(sql_state))
}

fn sanitize_error_message(message: &[u8]) -> String {
    String::from_utf8_lossy(message)
        .chars()
        .map(|value| if value.is_control() { ' ' } else { value })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlErrPacketParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
}

impl fmt::Display for MysqlErrPacketParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL ERR packet field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
        }
    }
}

impl Error for MysqlErrPacketParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_style_err_packet_summary() {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1096u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"HY000");
        payload.extend_from_slice(b"No tables used");

        let summary = parse_err_packet_summary(&payload)
            .expect("ERR packet should parse")
            .expect("payload should be an ERR packet");

        assert_eq!(
            summary,
            MysqlErrPacketSummary {
                error_code: 1096,
                sql_state: Some("HY000".to_owned()),
                message: "No tables used".to_owned(),
            }
        );
    }

    #[test]
    fn parses_err_packet_without_sql_state() {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1045u16.to_le_bytes());
        payload.extend_from_slice(b"Access denied");

        let summary = parse_err_packet_summary(&payload)
            .expect("ERR packet should parse")
            .expect("payload should be an ERR packet");

        assert_eq!(
            summary,
            MysqlErrPacketSummary {
                error_code: 1045,
                sql_state: None,
                message: "Access denied".to_owned(),
            }
        );
    }

    #[test]
    fn returns_none_for_non_err_payload() {
        let summary = parse_err_packet_summary(&[0x00]).expect("non-ERR payload should parse");

        assert_eq!(summary, None);
    }

    #[test]
    fn rejects_empty_payload() {
        let error = parse_err_packet_summary(&[]).expect_err("header should be missing");

        assert_eq!(
            error,
            MysqlErrPacketParseError::IncompletePayload {
                field: "header",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_incomplete_error_code() {
        let error =
            parse_err_packet_summary(&[0xff, 0x48]).expect_err("error code should be incomplete");

        assert_eq!(
            error,
            MysqlErrPacketParseError::IncompletePayload {
                field: "error_code",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_incomplete_sql_state() {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1096u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"HY");

        let error = parse_err_packet_summary(&payload).expect_err("SQLSTATE should be incomplete");

        assert_eq!(
            error,
            MysqlErrPacketParseError::IncompletePayload {
                field: "sql_state",
                needed: 5,
                available: 2,
            }
        );
    }

    #[test]
    fn decodes_message_lossily() {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1064u16.to_le_bytes());
        payload.push(0xff);

        let summary = parse_err_packet_summary(&payload)
            .expect("ERR packet should parse")
            .expect("payload should be an ERR packet");

        assert_eq!(summary.message, "\u{fffd}");
    }

    #[test]
    fn sanitizes_control_characters_in_message() {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1064u16.to_le_bytes());
        payload.extend_from_slice(b"bad\nquery\t\0message");

        let summary = parse_err_packet_summary(&payload)
            .expect("ERR packet should parse")
            .expect("payload should be an ERR packet");

        assert_eq!(summary.message, "bad query  message");
        assert!(!summary.message.chars().any(char::is_control));
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlErrPacketParseError::IncompletePayload {
                field: "error_code",
                needed: 2,
                available: 1,
            }
            .to_string(),
            "incomplete MySQL ERR packet field `error_code`: needed 2 bytes, available 1 bytes"
        );
    }
}
