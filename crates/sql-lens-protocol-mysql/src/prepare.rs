use std::{error::Error, fmt};

use crate::err::{MysqlErrPacketParseError, MysqlErrPacketSummary, parse_err_packet_summary};

const MYSQL_COM_STMT_PREPARE_OK_HEADER: u8 = 0x00;
const MYSQL_ERR_PACKET_HEADER: u8 = 0xff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlComStmtPrepareOk {
    pub statement_id: u32,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlComStmtPrepareResponse {
    Ok(MysqlComStmtPrepareOk),
    Error(MysqlErrPacketSummary),
}

pub fn parse_com_stmt_prepare_response(
    payload: &[u8],
) -> Result<Option<MysqlComStmtPrepareResponse>, MysqlComStmtPrepareResponseParseError> {
    let Some((&header, prepare_ok_payload)) = payload.split_first() else {
        return Err(MysqlComStmtPrepareResponseParseError::IncompletePayload {
            field: "header",
            needed: 1,
            available: 0,
        });
    };

    match header {
        MYSQL_COM_STMT_PREPARE_OK_HEADER => {
            let mut offset = 0;
            let statement_id = read_u32_le("statement_id", prepare_ok_payload, &mut offset)?;
            let num_columns = read_u16_le("num_columns", prepare_ok_payload, &mut offset)?;
            let num_params = read_u16_le("num_params", prepare_ok_payload, &mut offset)?;
            read_filler(prepare_ok_payload, &mut offset)?;
            let warning_count = read_optional_warning_count(prepare_ok_payload, &mut offset)?;

            Ok(Some(MysqlComStmtPrepareResponse::Ok(
                MysqlComStmtPrepareOk {
                    statement_id,
                    num_columns,
                    num_params,
                    warning_count,
                },
            )))
        }
        MYSQL_ERR_PACKET_HEADER => parse_err_packet_summary(payload)
            .map(|summary| summary.map(MysqlComStmtPrepareResponse::Error))
            .map_err(|source| MysqlComStmtPrepareResponseParseError::ErrPacket { source }),
        _ => Ok(None),
    }
}

fn read_u32_le(
    field: &'static str,
    payload: &[u8],
    offset: &mut usize,
) -> Result<u32, MysqlComStmtPrepareResponseParseError> {
    if payload.len().saturating_sub(*offset) < 4 {
        return Err(MysqlComStmtPrepareResponseParseError::IncompletePayload {
            field,
            needed: 4,
            available: payload.len().saturating_sub(*offset),
        });
    }

    let value = u32::from_le_bytes([
        payload[*offset],
        payload[*offset + 1],
        payload[*offset + 2],
        payload[*offset + 3],
    ]);
    *offset += 4;

    Ok(value)
}

fn read_u16_le(
    field: &'static str,
    payload: &[u8],
    offset: &mut usize,
) -> Result<u16, MysqlComStmtPrepareResponseParseError> {
    if payload.len().saturating_sub(*offset) < 2 {
        return Err(MysqlComStmtPrepareResponseParseError::IncompletePayload {
            field,
            needed: 2,
            available: payload.len().saturating_sub(*offset),
        });
    }

    let value = u16::from_le_bytes([payload[*offset], payload[*offset + 1]]);
    *offset += 2;

    Ok(value)
}

fn read_filler(
    payload: &[u8],
    offset: &mut usize,
) -> Result<(), MysqlComStmtPrepareResponseParseError> {
    if payload.len().saturating_sub(*offset) < 1 {
        return Err(MysqlComStmtPrepareResponseParseError::IncompletePayload {
            field: "filler",
            needed: 1,
            available: 0,
        });
    }

    *offset += 1;

    Ok(())
}

fn read_optional_warning_count(
    payload: &[u8],
    offset: &mut usize,
) -> Result<Option<u16>, MysqlComStmtPrepareResponseParseError> {
    let available = payload.len().saturating_sub(*offset);
    match available {
        0 => Ok(None),
        1 => Err(MysqlComStmtPrepareResponseParseError::IncompletePayload {
            field: "warning_count",
            needed: 2,
            available,
        }),
        _ => read_u16_le("warning_count", payload, offset).map(Some),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlComStmtPrepareResponseParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    ErrPacket {
        source: MysqlErrPacketParseError,
    },
}

impl fmt::Display for MysqlComStmtPrepareResponseParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL COM_STMT_PREPARE response field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::ErrPacket { source } => {
                write!(f, "invalid MySQL COM_STMT_PREPARE ERR response: {source}")
            }
        }
    }
}

impl Error for MysqlComStmtPrepareResponseParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IncompletePayload { .. } => None,
            Self::ErrPacket { source } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prepare_ok_response() {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE_OK_HEADER];
        payload.extend_from_slice(&0x1122_3344u32.to_le_bytes());
        payload.extend_from_slice(&3u16.to_le_bytes());
        payload.extend_from_slice(&2u16.to_le_bytes());
        payload.push(0x00);
        payload.extend_from_slice(&7u16.to_le_bytes());

        let response = parse_com_stmt_prepare_response(&payload)
            .expect("prepare response should parse")
            .expect("payload should be a prepare response");

        assert_eq!(
            response,
            MysqlComStmtPrepareResponse::Ok(MysqlComStmtPrepareOk {
                statement_id: 0x1122_3344,
                num_columns: 3,
                num_params: 2,
                warning_count: Some(7),
            })
        );
    }

    #[test]
    fn parses_prepare_ok_response_without_warning_count() {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE_OK_HEADER];
        payload.extend_from_slice(&42u32.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.extend_from_slice(&1u16.to_le_bytes());
        payload.push(0x00);

        let response = parse_com_stmt_prepare_response(&payload)
            .expect("prepare response should parse")
            .expect("payload should be a prepare response");

        assert_eq!(
            response,
            MysqlComStmtPrepareResponse::Ok(MysqlComStmtPrepareOk {
                statement_id: 42,
                num_columns: 0,
                num_params: 1,
                warning_count: None,
            })
        );
    }

    #[test]
    fn parses_prepare_err_response() {
        let mut payload = vec![MYSQL_ERR_PACKET_HEADER];
        payload.extend_from_slice(&1064u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"42000");
        payload.extend_from_slice(b"You have an error");

        let response = parse_com_stmt_prepare_response(&payload)
            .expect("prepare ERR response should parse")
            .expect("payload should be a prepare response");

        assert_eq!(
            response,
            MysqlComStmtPrepareResponse::Error(MysqlErrPacketSummary {
                error_code: 1064,
                sql_state: Some("42000".to_owned()),
                message: "You have an error".to_owned(),
            })
        );
    }

    #[test]
    fn returns_none_for_unrecognized_response() {
        let response =
            parse_com_stmt_prepare_response(&[0x01]).expect("unrecognized response is non-fatal");

        assert_eq!(response, None);
    }

    #[test]
    fn rejects_empty_payload() {
        let error = parse_com_stmt_prepare_response(&[]).expect_err("header should be missing");

        assert_eq!(
            error,
            MysqlComStmtPrepareResponseParseError::IncompletePayload {
                field: "header",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_incomplete_statement_id() {
        let error = parse_com_stmt_prepare_response(&[MYSQL_COM_STMT_PREPARE_OK_HEADER, 0x01])
            .expect_err("statement ID should be incomplete");

        assert_eq!(
            error,
            MysqlComStmtPrepareResponseParseError::IncompletePayload {
                field: "statement_id",
                needed: 4,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_incomplete_warning_count() {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE_OK_HEADER];
        payload.extend_from_slice(&42u32.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.extend_from_slice(&1u16.to_le_bytes());
        payload.push(0x00);
        payload.push(0x01);

        let error =
            parse_com_stmt_prepare_response(&payload).expect_err("warning count is incomplete");

        assert_eq!(
            error,
            MysqlComStmtPrepareResponseParseError::IncompletePayload {
                field: "warning_count",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn wraps_err_packet_parse_errors() {
        let error = parse_com_stmt_prepare_response(&[MYSQL_ERR_PACKET_HEADER, 0x48])
            .expect_err("ERR packet should be incomplete");

        assert_eq!(
            error,
            MysqlComStmtPrepareResponseParseError::ErrPacket {
                source: MysqlErrPacketParseError::IncompletePayload {
                    field: "error_code",
                    needed: 2,
                    available: 1,
                },
            }
        );
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlComStmtPrepareResponseParseError::IncompletePayload {
                field: "statement_id",
                needed: 4,
                available: 1,
            }
            .to_string(),
            "incomplete MySQL COM_STMT_PREPARE response field `statement_id`: needed 4 bytes, available 1 bytes"
        );
        assert_eq!(
            MysqlComStmtPrepareResponseParseError::ErrPacket {
                source: MysqlErrPacketParseError::IncompletePayload {
                    field: "error_code",
                    needed: 2,
                    available: 1,
                },
            }
            .to_string(),
            "invalid MySQL COM_STMT_PREPARE ERR response: incomplete MySQL ERR packet field `error_code`: needed 2 bytes, available 1 bytes"
        );
    }
}
