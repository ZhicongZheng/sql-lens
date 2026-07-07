use std::{error::Error, fmt, str};

const PROTOCOL_VERSION_10: u8 = 10;
const AUTH_PLUGIN_DATA_PART_1_LEN: usize = 8;
const FILLER_LEN: usize = 1;
const RESERVED_LEN: usize = 10;
const DEFAULT_AUTH_PLUGIN_DATA_PART_2_LEN: usize = 13;
const CLIENT_HANDSHAKE_RESERVED_LEN: usize = 23;
const CLIENT_CONNECT_WITH_DB: u32 = 0x0000_0008;
const CLIENT_PROTOCOL_41: u32 = 0x0000_0200;
const CLIENT_SSL: u32 = 0x0000_0800;
const CLIENT_SECURE_CONNECTION: u32 = 0x0000_8000;
const CLIENT_PLUGIN_AUTH: u32 = 0x0008_0000;
const CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x0020_0000;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlClientHandshakeResponse {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: Option<String>,
    pub database: Option<String>,
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

pub fn parse_client_handshake_response(
    payload: &[u8],
) -> Result<MysqlClientHandshakeResponse, MysqlClientHandshakeParseError> {
    let mut offset = 0;
    let capability_flags = read_client_u32_le(payload, &mut offset, "capability_flags")?;
    let max_packet_size = read_client_u32_le(payload, &mut offset, "max_packet_size")?;
    let character_set = read_client_u8(payload, &mut offset, "character_set")?;
    skip_client_required(
        payload,
        &mut offset,
        CLIENT_HANDSHAKE_RESERVED_LEN,
        "reserved",
    )?;

    if capability_flags & CLIENT_PROTOCOL_41 == 0 {
        return Err(MysqlClientHandshakeParseError::UnsupportedProtocol {
            message: "client handshake response does not advertise CLIENT_PROTOCOL_41",
        });
    }

    if capability_flags & CLIENT_SSL != 0 && offset == payload.len() {
        return Err(MysqlClientHandshakeParseError::UnsupportedProtocol {
            message: "SSLRequest packets are not full client handshake responses",
        });
    }

    let username = read_client_null_terminated_string(payload, &mut offset, "username")?;
    skip_auth_response(payload, &mut offset, capability_flags)?;

    let database = if capability_flags & CLIENT_CONNECT_WITH_DB != 0 {
        read_client_null_terminated_string(payload, &mut offset, "database")?
    } else {
        None
    };

    let auth_plugin_name = if capability_flags & CLIENT_PLUGIN_AUTH != 0 {
        read_client_null_terminated_string(payload, &mut offset, "auth_plugin_name")?
    } else {
        None
    };

    Ok(MysqlClientHandshakeResponse {
        capability_flags,
        max_packet_size,
        character_set,
        username,
        database,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlClientHandshakeParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    UnsupportedProtocol {
        message: &'static str,
    },
    MissingNullTerminator {
        field: &'static str,
    },
    InvalidUtf8 {
        field: &'static str,
    },
    InvalidLengthEncodedInteger {
        field: &'static str,
    },
}

impl fmt::Display for MysqlClientHandshakeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL client handshake response field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::UnsupportedProtocol { message } => {
                write!(f, "unsupported MySQL client handshake response: {message}")
            }
            Self::MissingNullTerminator { field } => write!(
                f,
                "missing NUL terminator in MySQL client handshake response field `{field}`"
            ),
            Self::InvalidUtf8 { field } => write!(
                f,
                "invalid UTF-8 in MySQL client handshake response field `{field}`"
            ),
            Self::InvalidLengthEncodedInteger { field } => write!(
                f,
                "invalid length-encoded integer in MySQL client handshake response field `{field}`"
            ),
        }
    }
}

impl Error for MysqlClientHandshakeParseError {}

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

fn skip_auth_response(
    payload: &[u8],
    offset: &mut usize,
    capability_flags: u32,
) -> Result<(), MysqlClientHandshakeParseError> {
    if capability_flags & CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA != 0 {
        let len = read_length_encoded_integer(payload, offset, "auth_response")?;
        let len = usize::try_from(len).map_err(|_| {
            MysqlClientHandshakeParseError::InvalidLengthEncodedInteger {
                field: "auth_response",
            }
        })?;

        return skip_client_required(payload, offset, len, "auth_response");
    }

    if capability_flags & CLIENT_SECURE_CONNECTION != 0 {
        let len = usize::from(read_client_u8(payload, offset, "auth_response_length")?);

        return skip_client_required(payload, offset, len, "auth_response");
    }

    skip_client_null_terminated_bytes(payload, offset, "auth_response")
}

fn read_length_encoded_integer(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u64, MysqlClientHandshakeParseError> {
    let first = read_client_u8(payload, offset, field)?;

    match first {
        0x00..=0xfa => Ok(u64::from(first)),
        0xfb | 0xff => Err(MysqlClientHandshakeParseError::InvalidLengthEncodedInteger { field }),
        0xfc => {
            let bytes = read_client_required(payload, offset, 2, field)?;

            Ok(u64::from(u16::from_le_bytes(
                bytes
                    .try_into()
                    .expect("read_client_required returned exactly 2 bytes"),
            )))
        }
        0xfd => {
            let bytes = read_client_required(payload, offset, 3, field)?;

            Ok(u64::from(bytes[0]) | (u64::from(bytes[1]) << 8) | (u64::from(bytes[2]) << 16))
        }
        0xfe => {
            let bytes = read_client_required(payload, offset, 8, field)?;

            Ok(u64::from_le_bytes(
                bytes
                    .try_into()
                    .expect("read_client_required returned exactly 8 bytes"),
            ))
        }
    }
}

fn read_client_null_terminated_string(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<Option<String>, MysqlClientHandshakeParseError> {
    let bytes = read_client_null_terminated_bytes(payload, offset, field)?;

    if bytes.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        str::from_utf8(bytes)
            .map_err(|_| MysqlClientHandshakeParseError::InvalidUtf8 { field })?
            .to_owned(),
    ))
}

fn read_client_null_terminated_bytes<'a>(
    payload: &'a [u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<&'a [u8], MysqlClientHandshakeParseError> {
    let remaining = &payload[*offset..];
    let terminator = remaining
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(MysqlClientHandshakeParseError::MissingNullTerminator { field })?;
    let start = *offset;
    *offset += terminator + 1;

    Ok(&payload[start..start + terminator])
}

fn skip_client_null_terminated_bytes(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<(), MysqlClientHandshakeParseError> {
    read_client_null_terminated_bytes(payload, offset, field).map(|_| ())
}

fn read_client_u32_le(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u32, MysqlClientHandshakeParseError> {
    let bytes = read_client_required(payload, offset, 4, field)?;

    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .expect("read_client_required returned exactly 4 bytes"),
    ))
}

fn read_client_u8(
    payload: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u8, MysqlClientHandshakeParseError> {
    let bytes = read_client_required(payload, offset, 1, field)?;

    Ok(bytes[0])
}

fn skip_client_required(
    payload: &[u8],
    offset: &mut usize,
    len: usize,
    field: &'static str,
) -> Result<(), MysqlClientHandshakeParseError> {
    read_client_required(payload, offset, len, field).map(|_| ())
}

fn read_client_required<'a>(
    payload: &'a [u8],
    offset: &mut usize,
    len: usize,
    field: &'static str,
) -> Result<&'a [u8], MysqlClientHandshakeParseError> {
    let available = payload.len().saturating_sub(*offset);

    if available < len {
        return Err(MysqlClientHandshakeParseError::IncompletePayload {
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

    #[test]
    fn parses_protocol_41_client_handshake_response() {
        let payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: Some(b"app_db"),
            plugin_name: Some(b"mysql_native_password"),
            auth_response: b"secret-password",
            use_lenenc_auth: false,
        });

        let response =
            parse_client_handshake_response(&payload).expect("client response should parse");

        assert_eq!(
            response.capability_flags,
            CLIENT_PROTOCOL_41
                | CLIENT_SECURE_CONNECTION
                | CLIENT_CONNECT_WITH_DB
                | CLIENT_PLUGIN_AUTH
        );
        assert_eq!(response.max_packet_size, 16 * 1024 * 1024);
        assert_eq!(response.character_set, 0x21);
        assert_eq!(response.username, Some("app".to_owned()));
        assert_eq!(response.database, Some("app_db".to_owned()));
        assert_eq!(
            response.auth_plugin_name,
            Some("mysql_native_password".to_owned())
        );
    }

    #[test]
    fn parses_client_response_without_database_or_plugin_name() {
        let payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: None,
            plugin_name: None,
            auth_response: b"secret-password",
            use_lenenc_auth: false,
        });

        let response =
            parse_client_handshake_response(&payload).expect("client response should parse");

        assert_eq!(response.username, Some("app".to_owned()));
        assert_eq!(response.database, None);
        assert_eq!(response.auth_plugin_name, None);
    }

    #[test]
    fn skips_length_encoded_auth_response() {
        let payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: Some(b"app_db"),
            plugin_name: Some(b"caching_sha2_password"),
            auth_response: b"secret-password",
            use_lenenc_auth: true,
        });

        let response =
            parse_client_handshake_response(&payload).expect("client response should parse");

        assert_eq!(response.database, Some("app_db".to_owned()));
        assert_eq!(
            response.auth_plugin_name,
            Some("caching_sha2_password".to_owned())
        );
    }

    #[test]
    fn does_not_expose_client_auth_response_bytes() {
        let payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: Some(b"app_db"),
            plugin_name: Some(b"mysql_native_password"),
            auth_response: b"secret-password",
            use_lenenc_auth: false,
        });

        let response =
            parse_client_handshake_response(&payload).expect("client response should parse");
        let debug = format!("{response:?}");

        assert!(!debug.contains("secret-password"));
    }

    #[test]
    fn rejects_incomplete_client_fixed_header() {
        let error = parse_client_handshake_response(&[0; 4])
            .expect_err("client fixed header should be incomplete");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::IncompletePayload {
                field: "max_packet_size",
                needed: 4,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_client_response_without_protocol_41() {
        let mut payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: None,
            plugin_name: None,
            auth_response: b"secret-password",
            use_lenenc_auth: false,
        });
        payload[0] = 0;
        payload[1] = 0;
        payload[2] = 0;
        payload[3] = 0;

        let error =
            parse_client_handshake_response(&payload).expect_err("protocol 41 flag is missing");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::UnsupportedProtocol {
                message: "client handshake response does not advertise CLIENT_PROTOCOL_41",
            }
        );
    }

    #[test]
    fn rejects_ssl_request_as_full_client_response() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(CLIENT_PROTOCOL_41 | CLIENT_SSL).to_le_bytes());
        payload.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&[0; CLIENT_HANDSHAKE_RESERVED_LEN]);

        let error =
            parse_client_handshake_response(&payload).expect_err("SSLRequest should be deferred");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::UnsupportedProtocol {
                message: "SSLRequest packets are not full client handshake responses",
            }
        );
    }

    #[test]
    fn rejects_missing_client_username_terminator() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION).to_le_bytes());
        payload.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&[0; CLIENT_HANDSHAKE_RESERVED_LEN]);
        payload.extend_from_slice(b"app");

        let error = parse_client_handshake_response(&payload)
            .expect_err("username terminator should be missing");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::MissingNullTerminator { field: "username" }
        );
    }

    #[test]
    fn rejects_invalid_utf8_client_database() {
        let payload = client_handshake_response_payload(ClientHandshakeOptions {
            username: b"app",
            database: Some(&[0xff]),
            plugin_name: Some(b"mysql_native_password"),
            auth_response: b"secret-password",
            use_lenenc_auth: false,
        });

        let error = parse_client_handshake_response(&payload)
            .expect_err("database should be invalid UTF-8");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::InvalidUtf8 { field: "database" }
        );
    }

    #[test]
    fn rejects_invalid_length_encoded_auth_response() {
        let mut payload = Vec::new();
        payload.extend_from_slice(
            &(CLIENT_PROTOCOL_41
                | CLIENT_CONNECT_WITH_DB
                | CLIENT_PLUGIN_AUTH
                | CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA)
                .to_le_bytes(),
        );
        payload.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&[0; CLIENT_HANDSHAKE_RESERVED_LEN]);
        payload.extend_from_slice(b"app");
        payload.push(0);
        payload.push(0xfb);

        let error = parse_client_handshake_response(&payload)
            .expect_err("length-encoded auth response should be invalid");

        assert_eq!(
            error,
            MysqlClientHandshakeParseError::InvalidLengthEncodedInteger {
                field: "auth_response"
            }
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

    struct ClientHandshakeOptions<'a> {
        username: &'a [u8],
        database: Option<&'a [u8]>,
        plugin_name: Option<&'a [u8]>,
        auth_response: &'a [u8],
        use_lenenc_auth: bool,
    }

    fn client_handshake_response_payload(options: ClientHandshakeOptions<'_>) -> Vec<u8> {
        let mut capability_flags = CLIENT_PROTOCOL_41;

        if options.use_lenenc_auth {
            capability_flags |= CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA;
        } else {
            capability_flags |= CLIENT_SECURE_CONNECTION;
        }
        if options.database.is_some() {
            capability_flags |= CLIENT_CONNECT_WITH_DB;
        }
        if options.plugin_name.is_some() {
            capability_flags |= CLIENT_PLUGIN_AUTH;
        }

        let mut payload = Vec::new();
        payload.extend_from_slice(&capability_flags.to_le_bytes());
        payload.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&[0; CLIENT_HANDSHAKE_RESERVED_LEN]);
        payload.extend_from_slice(options.username);
        payload.push(0);

        if options.use_lenenc_auth {
            payload.push(
                u8::try_from(options.auth_response.len())
                    .expect("test auth response length should fit one lenenc byte"),
            );
        } else {
            payload.push(
                u8::try_from(options.auth_response.len())
                    .expect("test auth response length should fit one byte"),
            );
        }
        payload.extend_from_slice(options.auth_response);

        if let Some(database) = options.database {
            payload.extend_from_slice(database);
            payload.push(0);
        }
        if let Some(plugin_name) = options.plugin_name {
            payload.extend_from_slice(plugin_name);
            payload.push(0);
        }

        payload
    }
}
