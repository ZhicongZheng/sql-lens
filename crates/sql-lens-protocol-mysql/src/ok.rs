use std::{error::Error, fmt};

const MYSQL_OK_PACKET_HEADER: u8 = 0x00;
const LENENC_TWO_BYTE_MARKER: u8 = 0xfc;
const LENENC_THREE_BYTE_MARKER: u8 = 0xfd;
const LENENC_EIGHT_BYTE_MARKER: u8 = 0xfe;
const LENENC_NULL_MARKER: u8 = 0xfb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlOkPacketSummary {
    pub affected_rows: u64,
    pub status_flags: Option<u16>,
}

pub fn parse_ok_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlOkPacketSummary>, MysqlOkPacketParseError> {
    let Some((&header, payload)) = payload.split_first() else {
        return Err(MysqlOkPacketParseError::IncompletePayload {
            field: "header",
            needed: 1,
            available: 0,
        });
    };

    if header != MYSQL_OK_PACKET_HEADER {
        return Ok(None);
    }

    let (affected_rows, affected_rows_len) = read_lenenc_integer("affected_rows", payload)?;
    let payload = &payload[affected_rows_len..];
    let (_, last_insert_id_len) = read_lenenc_integer("last_insert_id", payload)?;
    let payload = &payload[last_insert_id_len..];
    let status_flags = if payload.len() >= 2 {
        Some(u16::from_le_bytes([payload[0], payload[1]]))
    } else {
        None
    };

    Ok(Some(MysqlOkPacketSummary {
        affected_rows,
        status_flags,
    }))
}

fn read_lenenc_integer(
    field: &'static str,
    input: &[u8],
) -> Result<(u64, usize), MysqlOkPacketParseError> {
    let Some((&first, rest)) = input.split_first() else {
        return Err(MysqlOkPacketParseError::IncompletePayload {
            field,
            needed: 1,
            available: 0,
        });
    };

    match first {
        0x00..=0xfa => Ok((u64::from(first), 1)),
        LENENC_NULL_MARKER => Err(MysqlOkPacketParseError::InvalidLengthEncodedInteger {
            field,
            marker: LENENC_NULL_MARKER,
        }),
        LENENC_TWO_BYTE_MARKER => read_fixed_lenenc_integer(field, rest, 2).map(|value| (value, 3)),
        LENENC_THREE_BYTE_MARKER => {
            read_fixed_lenenc_integer(field, rest, 3).map(|value| (value, 4))
        }
        LENENC_EIGHT_BYTE_MARKER => {
            read_fixed_lenenc_integer(field, rest, 8).map(|value| (value, 9))
        }
        marker => Err(MysqlOkPacketParseError::InvalidLengthEncodedInteger { field, marker }),
    }
}

fn read_fixed_lenenc_integer(
    field: &'static str,
    input: &[u8],
    len: usize,
) -> Result<u64, MysqlOkPacketParseError> {
    if input.len() < len {
        return Err(MysqlOkPacketParseError::IncompletePayload {
            field,
            needed: len,
            available: input.len(),
        });
    }

    let mut value = 0u64;
    for (index, byte) in input[..len].iter().enumerate() {
        value |= u64::from(*byte) << (index * 8);
    }

    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlOkPacketParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    InvalidLengthEncodedInteger {
        field: &'static str,
        marker: u8,
    },
}

impl fmt::Display for MysqlOkPacketParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL OK packet field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::InvalidLengthEncodedInteger { field, marker } => write!(
                f,
                "invalid length-encoded integer marker 0x{marker:02x} in MySQL OK packet field `{field}`"
            ),
        }
    }
}

impl Error for MysqlOkPacketParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_style_ok_packet_summary() {
        let summary = parse_ok_packet_summary(&[0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00])
            .expect("OK packet should parse")
            .expect("payload should be an OK packet");

        assert_eq!(
            summary,
            MysqlOkPacketSummary {
                affected_rows: 0,
                status_flags: Some(0x0002),
            }
        );
    }

    #[test]
    fn parses_one_byte_affected_rows() {
        let summary = parse_ok_packet_summary(&[0x00, 0x2a, 0x00, 0x02, 0x00, 0x00, 0x00])
            .expect("OK packet should parse")
            .expect("payload should be an OK packet");

        assert_eq!(summary.affected_rows, 42);
        assert_eq!(summary.status_flags, Some(0x0002));
    }

    #[test]
    fn parses_two_byte_length_encoded_affected_rows() {
        let summary = parse_ok_packet_summary(&[0x00, 0xfc, 0x2c, 0x01, 0x00, 0x02, 0x00])
            .expect("OK packet should parse")
            .expect("payload should be an OK packet");

        assert_eq!(summary.affected_rows, 300);
        assert_eq!(summary.status_flags, Some(0x0002));
    }

    #[test]
    fn parses_three_byte_length_encoded_affected_rows() {
        let summary = parse_ok_packet_summary(&[0x00, 0xfd, 0x70, 0x11, 0x01, 0x00, 0x02, 0x00])
            .expect("OK packet should parse")
            .expect("payload should be an OK packet");

        assert_eq!(summary.affected_rows, 70_000);
        assert_eq!(summary.status_flags, Some(0x0002));
    }

    #[test]
    fn parses_eight_byte_length_encoded_affected_rows() {
        let summary = parse_ok_packet_summary(&[
            0x00, 0xfe, 0x15, 0xcd, 0x5b, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
        ])
        .expect("OK packet should parse")
        .expect("payload should be an OK packet");

        assert_eq!(summary.affected_rows, 123_456_789);
        assert_eq!(summary.status_flags, Some(0x0002));
    }

    #[test]
    fn returns_none_for_non_ok_payload() {
        let summary = parse_ok_packet_summary(&[0xff]).expect("non-OK payload should parse");

        assert_eq!(summary, None);
    }

    #[test]
    fn accepts_ok_packet_without_status_flags() {
        let summary = parse_ok_packet_summary(&[0x00, 0x01, 0x00])
            .expect("OK packet should parse")
            .expect("payload should be an OK packet");

        assert_eq!(
            summary,
            MysqlOkPacketSummary {
                affected_rows: 1,
                status_flags: None,
            }
        );
    }

    #[test]
    fn rejects_incomplete_header() {
        let error = parse_ok_packet_summary(&[]).expect_err("header should be missing");

        assert_eq!(
            error,
            MysqlOkPacketParseError::IncompletePayload {
                field: "header",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_incomplete_two_byte_length_encoded_integer() {
        let error =
            parse_ok_packet_summary(&[0x00, 0xfc, 0x2c]).expect_err("affected rows is incomplete");

        assert_eq!(
            error,
            MysqlOkPacketParseError::IncompletePayload {
                field: "affected_rows",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_lenenc_null_marker() {
        let error =
            parse_ok_packet_summary(&[0x00, 0xfb]).expect_err("affected rows marker is invalid");

        assert_eq!(
            error,
            MysqlOkPacketParseError::InvalidLengthEncodedInteger {
                field: "affected_rows",
                marker: 0xfb,
            }
        );
    }

    #[test]
    fn rejects_undefined_lenenc_marker() {
        let error =
            parse_ok_packet_summary(&[0x00, 0xff]).expect_err("affected rows marker is invalid");

        assert_eq!(
            error,
            MysqlOkPacketParseError::InvalidLengthEncodedInteger {
                field: "affected_rows",
                marker: 0xff,
            }
        );
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlOkPacketParseError::IncompletePayload {
                field: "affected_rows",
                needed: 2,
                available: 1,
            }
            .to_string(),
            "incomplete MySQL OK packet field `affected_rows`: needed 2 bytes, available 1 bytes"
        );
        assert_eq!(
            MysqlOkPacketParseError::InvalidLengthEncodedInteger {
                field: "affected_rows",
                marker: 0xfb,
            }
            .to_string(),
            "invalid length-encoded integer marker 0xfb in MySQL OK packet field `affected_rows`"
        );
    }
}
