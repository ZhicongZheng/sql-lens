use std::{error::Error, fmt, str};

const PROTOCOL_VERSION_10: u8 = 10;
const AUTH_PLUGIN_DATA_PART_1_LEN: usize = 8;
const FILLER_LEN: usize = 1;
const RESERVED_LEN: usize = 10;
const DEFAULT_AUTH_PLUGIN_DATA_PART_2_LEN: usize = 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlInitialHandshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub capability_flags: Option<u32>,
    pub character_set: Option<u8>,
    pub status_flags: Option<u16>,
    pub auth_plugin_name: Option<String>,
}

pub fn parse_initial_handshake(
    payload: &[u8],
) -> Result<MysqlInitialHandshake, MysqlHandshakeParseError> {
    let Some((&protocol_version, rest)) = payload.split_first() else {
        return Err(MysqlHandshakeParseError::EmptyPayload);
    };

    if protocol_version != PROTOCOL_VERSION_10 {
        return Err(MysqlHandshakeParseError::UnsupportedProtocolVersion {
            version: protocol_version,
        });
    }

    let server_version_end = rest
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(MysqlHandshakeParseError::MissingServerVersionTerminator)?;
    let server_version = parse_utf8(&rest[..server_version_end], "server_version")?.to_owned();
    let mut offset = 1 + server_version_end + 1;

    let connection_id = read_u32_le(payload, &mut offset, "connection_id")?;
    skip_required(
        payload,
        &mut offset,
        AUTH_PLUGIN_DATA_PART_1_LEN,
        "auth_plugin_data_part_1",
    )?;
    skip_required(payload, &mut offset, FILLER_LEN, "filler")?;

    let capability_flags_1 = read_optional_u16_le(payload, &mut offset, "capability_flags_1")?;
    let character_set = read_optional_u8(payload, &mut offset, "character_set")?;
    let status_flags = read_optional_u16_le(payload, &mut offset, "status_flags")?;
    let capability_flags_2 = read_optional_u16_le(payload, &mut offset, "capability_flags_2")?;
    let auth_plugin_data_len = read_optional_u8(payload, &mut offset, "auth_plugin_data_len")?;

    let capability_flags = capability_flags_1
        .map(|lower| u32::from(lower) | (u32::from(capability_flags_2.unwrap_or_default()) << 16));
    let auth_plugin_name = parse_auth_plugin_name(payload, offset, auth_plugin_data_len)?;

    Ok(MysqlInitialHandshake {
        protocol_version,
        server_version,
        connection_id,
        capability_flags,
        character_set,
        status_flags,
        auth_plugin_name,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlHandshakeParseError {
    EmptyPayload,
    UnsupportedProtocolVersion {
        version: u8,
    },
    MissingServerVersionTerminator,
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    InvalidUtf8 {
        field: &'static str,
    },
}

impl fmt::Display for MysqlHandshakeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => write!(f, "empty MySQL initial handshake payload"),
            Self::UnsupportedProtocolVersion { version } => {
                write!(
                    f,
                    "unsupported MySQL initial handshake protocol version: {version}"
                )
            }
            Self::MissingServerVersionTerminator => {
                write!(f, "missing MySQL server version terminator")
            }
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL initial handshake field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::InvalidUtf8 { field } => {
                write!(
                    f,
                    "invalid UTF-8 in MySQL initial handshake field `{field}`"
                )
            }
        }
    }
}

impl Error for MysqlHandshakeParseError {}

fn parse_auth_plugin_name(
    payload: &[u8],
    mut offset: usize,
    auth_plugin_data_len: Option<u8>,
) -> Result<Option<String>, MysqlHandshakeParseError> {
    if offset == payload.len() {
        return Ok(None);
    }

    skip_required(payload, &mut offset, RESERVED_LEN, "reserved")?;

    if offset == payload.len() {
        return Ok(None);
    }

    let part_2_len = auth_plugin_data_len
        .map(|length| {
            usize::from(length)
                .saturating_sub(AUTH_PLUGIN_DATA_PART_1_LEN)
                .max(DEFAULT_AUTH_PLUGIN_DATA_PART_2_LEN)
        })
        .unwrap_or(DEFAULT_AUTH_PLUGIN_DATA_PART_2_LEN);
    skip_required(payload, &mut offset, part_2_len, "auth_plugin_data_part_2")?;

    if offset == payload.len() {
        return Ok(None);
    }

    let remaining = &payload[offset..];
    let plugin_name_end = remaining.iter().position(|byte| *byte == 0).ok_or(
        MysqlHandshakeParseError::IncompletePayload {
            field: "auth_plugin_name",
            needed: remaining.len() + 1,
            available: remaining.len(),
        },
    )?;
    let plugin_name = &remaining[..plugin_name_end];

    if plugin_name.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        parse_utf8(plugin_name, "auth_plugin_name")?.to_owned(),
    ))
}

fn parse_utf8<'a>(
    value: &'a [u8],
    field: &'static str,
) -> Result<&'a str, MysqlHandshakeParseError> {
    str::from_utf8(value).map_err(|_| MysqlHandshakeParseError::InvalidUtf8 { field })
}

fn read_u32_le(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u32, MysqlHandshakeParseError> {
    let bytes = read_required(payload, offset, 4, field)?;

    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .expect("read_required returned exactly 4 bytes"),
    ))
}

fn read_optional_u16_le(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<Option<u16>, MysqlHandshakeParseError> {
    if *offset == payload.len() {
        return Ok(None);
    }

    let bytes = read_required(payload, offset, 2, field)?;

    Ok(Some(u16::from_le_bytes(
        bytes
            .try_into()
            .expect("read_required returned exactly 2 bytes"),
    )))
}

fn read_optional_u8(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<Option<u8>, MysqlHandshakeParseError> {
    if *offset == payload.len() {
        return Ok(None);
    }

    let bytes = read_required(payload, offset, 1, field)?;

    Ok(Some(bytes[0]))
}

fn skip_required(
    payload: &[u8],
    offset: &mut usize,
    len: usize,
    field: &'static str,
) -> Result<(), MysqlHandshakeParseError> {
    read_required(payload, offset, len, field).map(|_| ())
}

fn read_required<'a>(
    payload: &'a [u8],
    offset: &mut usize,
    len: usize,
    field: &'static str,
) -> Result<&'a [u8], MysqlHandshakeParseError> {
    let available = payload.len().saturating_sub(*offset);

    if available < len {
        return Err(MysqlHandshakeParseError::IncompletePayload {
            field,
            needed: len,
            available,
        });
    }

    let start = *offset;
    *offset += len;

    Ok(&payload[start..start + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_protocol_10_initial_handshake() {
        let payload = representative_handshake_payload();

        let handshake = parse_initial_handshake(&payload).expect("handshake should parse");

        assert_eq!(handshake.protocol_version, 10);
        assert_eq!(handshake.server_version, "8.0.36");
        assert_eq!(handshake.connection_id, 0x0102_0304);
        assert_eq!(handshake.capability_flags, Some(0x5678_1234));
        assert_eq!(handshake.character_set, Some(0x21));
        assert_eq!(handshake.status_flags, Some(0x0002));
        assert_eq!(
            handshake.auth_plugin_name,
            Some("mysql_native_password".to_owned())
        );
    }

    #[test]
    fn does_not_expose_auth_challenge_bytes() {
        let payload = representative_handshake_payload();

        let handshake = parse_initial_handshake(&payload).expect("handshake should parse");
        let debug = format!("{handshake:?}");

        assert!(!debug.contains("abcdefgh"));
        assert!(!debug.contains("ijklmnopqrst"));
    }

    #[test]
    fn rejects_empty_payload() {
        let error = parse_initial_handshake(&[]).expect_err("payload should be empty");

        assert_eq!(error, MysqlHandshakeParseError::EmptyPayload);
    }

    #[test]
    fn rejects_unsupported_protocol_version() {
        let error = parse_initial_handshake(&[9]).expect_err("version should be unsupported");

        assert_eq!(
            error,
            MysqlHandshakeParseError::UnsupportedProtocolVersion { version: 9 }
        );
    }

    #[test]
    fn rejects_missing_server_version_terminator() {
        let error =
            parse_initial_handshake(&[10, b'8', b'.', b'0']).expect_err("terminator is missing");

        assert_eq!(
            error,
            MysqlHandshakeParseError::MissingServerVersionTerminator
        );
    }

    #[test]
    fn rejects_incomplete_connection_id() {
        let error =
            parse_initial_handshake(&[10, b'8', 0, 0x01, 0x02]).expect_err("id is incomplete");

        assert_eq!(
            error,
            MysqlHandshakeParseError::IncompletePayload {
                field: "connection_id",
                needed: 4,
                available: 2,
            }
        );
    }

    #[test]
    fn rejects_invalid_server_version_utf8() {
        let error = parse_initial_handshake(&[10, 0xff, 0, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .expect_err("server version should be invalid UTF-8");

        assert_eq!(
            error,
            MysqlHandshakeParseError::InvalidUtf8 {
                field: "server_version"
            }
        );
    }

    #[test]
    fn parses_minimal_handshake_without_optional_fields() {
        let mut payload = Vec::from([10]);
        payload.extend_from_slice(b"5.7.0");
        payload.push(0);
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(b"abcdefgh");
        payload.push(0);

        let handshake = parse_initial_handshake(&payload).expect("minimal handshake should parse");

        assert_eq!(handshake.server_version, "5.7.0");
        assert_eq!(handshake.connection_id, 1);
        assert_eq!(handshake.capability_flags, None);
        assert_eq!(handshake.auth_plugin_name, None);
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlHandshakeParseError::EmptyPayload.to_string(),
            "empty MySQL initial handshake payload"
        );
        assert_eq!(
            MysqlHandshakeParseError::IncompletePayload {
                field: "connection_id",
                needed: 4,
                available: 2,
            }
            .to_string(),
            "incomplete MySQL initial handshake field `connection_id`: needed 4 bytes, available 2 bytes"
        );
    }

    fn representative_handshake_payload() -> Vec<u8> {
        let mut payload = Vec::new();

        payload.push(10);
        payload.extend_from_slice(b"8.0.36");
        payload.push(0);
        payload.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        payload.extend_from_slice(b"abcdefgh");
        payload.push(0);
        payload.extend_from_slice(&0x1234u16.to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&0x0002u16.to_le_bytes());
        payload.extend_from_slice(&0x5678u16.to_le_bytes());
        payload.push(21);
        payload.extend_from_slice(&[0; 10]);
        payload.extend_from_slice(b"ijklmnopqrst");
        payload.push(0);
        payload.extend_from_slice(b"mysql_native_password");
        payload.push(0);

        payload
    }
}
