use std::{error::Error, fmt};

pub const MYSQL_PACKET_HEADER_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlPacketHeader {
    pub payload_length: u32,
    pub sequence_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlPacket<'a> {
    pub header: MysqlPacketHeader,
    pub payload: &'a [u8],
}

pub fn parse_mysql_packet(input: &[u8]) -> Result<MysqlPacket<'_>, MysqlPacketParseError> {
    if input.len() < MYSQL_PACKET_HEADER_LEN {
        return Err(MysqlPacketParseError::IncompleteHeader {
            available: input.len(),
        });
    }

    let payload_length =
        u32::from(input[0]) | (u32::from(input[1]) << 8) | (u32::from(input[2]) << 16);
    let sequence_id = input[3];
    let available_payload = input.len() - MYSQL_PACKET_HEADER_LEN;
    let payload_length_usize =
        usize::try_from(payload_length).expect("3-byte payload length always fits usize");

    if available_payload < payload_length_usize {
        return Err(MysqlPacketParseError::IncompletePayload {
            declared: payload_length,
            available: available_payload,
        });
    }

    let payload_end = MYSQL_PACKET_HEADER_LEN + payload_length_usize;

    Ok(MysqlPacket {
        header: MysqlPacketHeader {
            payload_length,
            sequence_id,
        },
        payload: &input[MYSQL_PACKET_HEADER_LEN..payload_end],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlPacketParseError {
    IncompleteHeader { available: usize },
    IncompletePayload { declared: u32, available: usize },
}

impl fmt::Display for MysqlPacketParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteHeader { available } => write!(
                f,
                "incomplete MySQL packet header: available {available} of {MYSQL_PACKET_HEADER_LEN} bytes"
            ),
            Self::IncompletePayload {
                declared,
                available,
            } => write!(
                f,
                "incomplete MySQL packet payload: declared {declared} bytes, available {available} bytes"
            ),
        }
    }
}

impl Error for MysqlPacketParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet_fixture(path: &str, input: &str) -> Vec<u8> {
        input
            .lines()
            .flat_map(|line| {
                line.split_once('#')
                    .map_or(line, |(bytes, _)| bytes)
                    .split_whitespace()
            })
            .map(|byte| {
                u8::from_str_radix(byte, 16)
                    .unwrap_or_else(|error| panic!("invalid hex byte `{byte}` in {path}: {error}"))
            })
            .collect()
    }

    #[test]
    fn parses_normal_packet_header_and_payload() {
        let input = [0x03, 0x00, 0x00, 0x02, b'a', b'b', b'c'];

        let packet = parse_mysql_packet(&input).expect("packet should parse");

        assert_eq!(
            packet.header,
            MysqlPacketHeader {
                payload_length: 3,
                sequence_id: 2,
            }
        );
        assert_eq!(packet.payload, b"abc");
    }

    #[test]
    fn parses_empty_payload_packet() {
        let input = [0x00, 0x00, 0x00, 0x07];

        let packet = parse_mysql_packet(&input).expect("packet should parse");

        assert_eq!(
            packet.header,
            MysqlPacketHeader {
                payload_length: 0,
                sequence_id: 7,
            }
        );
        assert_eq!(packet.payload, b"");
    }

    #[test]
    fn parses_three_byte_little_endian_payload_length() {
        let input = [0x01, 0x02, 0x03, 0x04];

        let error = parse_mysql_packet(&input).expect_err("payload should be incomplete");

        assert_eq!(
            error,
            MysqlPacketParseError::IncompletePayload {
                declared: 0x03_02_01,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_short_headers() {
        for available in 0..MYSQL_PACKET_HEADER_LEN {
            let input = vec![0; available];

            let error = parse_mysql_packet(&input).expect_err("header should be incomplete");

            assert_eq!(error, MysqlPacketParseError::IncompleteHeader { available });
        }
    }

    #[test]
    fn rejects_incomplete_payload() {
        let input = [0x05, 0x00, 0x00, 0x01, b'a', b'b'];

        let error = parse_mysql_packet(&input).expect_err("payload should be incomplete");

        assert_eq!(
            error,
            MysqlPacketParseError::IncompletePayload {
                declared: 5,
                available: 2,
            }
        );
    }

    #[test]
    fn ignores_trailing_bytes_after_first_packet() {
        let input = [0x02, 0x00, 0x00, 0x01, b'o', b'k', b'x', b'y'];

        let packet = parse_mysql_packet(&input).expect("packet should parse");

        assert_eq!(packet.payload, b"ok");
    }

    #[test]
    fn parse_errors_have_clear_display_messages() {
        assert_eq!(
            MysqlPacketParseError::IncompleteHeader { available: 2 }.to_string(),
            "incomplete MySQL packet header: available 2 of 4 bytes"
        );
        assert_eq!(
            MysqlPacketParseError::IncompletePayload {
                declared: 8,
                available: 3,
            }
            .to_string(),
            "incomplete MySQL packet payload: declared 8 bytes, available 3 bytes"
        );
    }

    #[test]
    fn parses_normal_packet_fixture() {
        let input = packet_fixture(
            "fixtures/packets/normal.hex",
            include_str!("../fixtures/packets/normal.hex"),
        );

        let packet = parse_mysql_packet(&input).expect("fixture packet should parse");

        assert_eq!(packet.header.payload_length, 3);
        assert_eq!(packet.header.sequence_id, 2);
        assert_eq!(packet.payload, b"abc");
    }

    #[test]
    fn parses_empty_payload_packet_fixture() {
        let input = packet_fixture(
            "fixtures/packets/empty-payload.hex",
            include_str!("../fixtures/packets/empty-payload.hex"),
        );

        let packet = parse_mysql_packet(&input).expect("fixture packet should parse");

        assert_eq!(packet.header.payload_length, 0);
        assert_eq!(packet.header.sequence_id, 7);
        assert_eq!(packet.payload, b"");
    }

    #[test]
    fn rejects_short_header_packet_fixture() {
        let input = packet_fixture(
            "fixtures/packets/malformed-short-header.hex",
            include_str!("../fixtures/packets/malformed-short-header.hex"),
        );

        let error = parse_mysql_packet(&input).expect_err("fixture should be malformed");

        assert_eq!(
            error,
            MysqlPacketParseError::IncompleteHeader { available: 3 }
        );
    }

    #[test]
    fn rejects_incomplete_payload_packet_fixture() {
        let input = packet_fixture(
            "fixtures/packets/malformed-incomplete-payload.hex",
            include_str!("../fixtures/packets/malformed-incomplete-payload.hex"),
        );

        let error = parse_mysql_packet(&input).expect_err("fixture should be malformed");

        assert_eq!(
            error,
            MysqlPacketParseError::IncompletePayload {
                declared: 5,
                available: 2,
            }
        );
    }
}
