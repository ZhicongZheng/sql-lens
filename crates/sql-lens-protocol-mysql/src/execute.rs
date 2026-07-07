use std::{error::Error, fmt};

use sql_lens_core::SqlParameterValue;

const MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT: u8 = 0x01;
const MYSQL_PARAMETER_FLAG_UNSIGNED: u8 = 0x80;

const MYSQL_TYPE_TINY: u8 = 0x01;
const MYSQL_TYPE_SHORT: u8 = 0x02;
const MYSQL_TYPE_LONG: u8 = 0x03;
const MYSQL_TYPE_FLOAT: u8 = 0x04;
const MYSQL_TYPE_DOUBLE: u8 = 0x05;
const MYSQL_TYPE_TIMESTAMP: u8 = 0x07;
const MYSQL_TYPE_LONGLONG: u8 = 0x08;
const MYSQL_TYPE_INT24: u8 = 0x09;
const MYSQL_TYPE_DATE: u8 = 0x0a;
const MYSQL_TYPE_TIME: u8 = 0x0b;
const MYSQL_TYPE_DATETIME: u8 = 0x0c;
const MYSQL_TYPE_NEWDATE: u8 = 0x0e;
const MYSQL_TYPE_VARCHAR: u8 = 0x0f;
const MYSQL_TYPE_BIT: u8 = 0x10;
const MYSQL_TYPE_ENUM: u8 = 0xf7;
const MYSQL_TYPE_SET: u8 = 0xf8;
const MYSQL_TYPE_TINY_BLOB: u8 = 0xf9;
const MYSQL_TYPE_MEDIUM_BLOB: u8 = 0xfa;
const MYSQL_TYPE_LONG_BLOB: u8 = 0xfb;
const MYSQL_TYPE_BLOB: u8 = 0xfc;
const MYSQL_TYPE_VAR_STRING: u8 = 0xfd;
const MYSQL_TYPE_STRING: u8 = 0xfe;
const MYSQL_TYPE_GEOMETRY: u8 = 0xff;

const MYSQL_LENGTH_ENCODED_NULL_MARKER: u8 = 0xfb;
const MYSQL_LENGTH_ENCODED_U16_MARKER: u8 = 0xfc;
const MYSQL_LENGTH_ENCODED_U24_MARKER: u8 = 0xfd;
const MYSQL_LENGTH_ENCODED_U64_MARKER: u8 = 0xfe;
const BINARY_SUMMARY_HEX_PREFIX_BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlNullBitmap {
    pub null_parameter_indexes: Vec<usize>,
    pub bytes_consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlParameterType {
    pub type_code: u8,
    pub unsigned: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MysqlDecodedParameter {
    pub index: u16,
    pub value: SqlParameterValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MysqlDecodedParameters {
    pub parameters: Vec<MysqlDecodedParameter>,
    pub bytes_consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlExpandedSqlRenderError {
    MissingParameter {
        placeholder_index: usize,
        parameter_count: usize,
    },
    ExtraParameters {
        placeholder_count: usize,
        parameter_count: usize,
    },
}

impl fmt::Display for MysqlExpandedSqlRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingParameter {
                placeholder_index,
                parameter_count,
            } => write!(
                f,
                "missing MySQL prepared statement parameter for placeholder {placeholder_index}; only {parameter_count} parameters decoded"
            ),
            Self::ExtraParameters {
                placeholder_count,
                parameter_count,
            } => write!(
                f,
                "too many MySQL prepared statement parameters: {parameter_count} decoded for {placeholder_count} placeholders"
            ),
        }
    }
}

impl Error for MysqlExpandedSqlRenderError {}

pub fn render_expanded_sql(
    template_sql: &str,
    parameters: &[MysqlDecodedParameter],
) -> Result<String, MysqlExpandedSqlRenderError> {
    let bytes = template_sql.as_bytes();
    let mut output = String::with_capacity(template_sql.len());
    let mut parameter_index = 0;
    let mut last_emitted = 0;
    let mut cursor = 0;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\'' | b'"' | b'`' => {
                cursor = skip_quoted_sql(bytes, cursor, bytes[cursor]);
            }
            b'-' if starts_line_comment(bytes, cursor) => {
                cursor = skip_line_comment(bytes, cursor);
            }
            b'#' => {
                cursor = skip_line_comment(bytes, cursor);
            }
            b'/' if starts_block_comment(bytes, cursor) => {
                cursor = skip_block_comment(bytes, cursor);
            }
            b'?' => {
                let Some(parameter) = parameters.get(parameter_index) else {
                    return Err(MysqlExpandedSqlRenderError::MissingParameter {
                        placeholder_index: parameter_index,
                        parameter_count: parameters.len(),
                    });
                };
                output.push_str(&template_sql[last_emitted..cursor]);
                output.push_str(&render_parameter_literal(&parameter.value));
                parameter_index += 1;
                cursor += 1;
                last_emitted = cursor;
            }
            _ => {
                cursor += 1;
            }
        }
    }

    if parameter_index < parameters.len() {
        return Err(MysqlExpandedSqlRenderError::ExtraParameters {
            placeholder_count: parameter_index,
            parameter_count: parameters.len(),
        });
    }

    output.push_str(&template_sql[last_emitted..]);
    Ok(output)
}

pub fn decode_null_bitmap(
    parameter_payload: &[u8],
    parameter_count: u16,
) -> Result<MysqlNullBitmap, MysqlExecuteParseError> {
    let bitmap_len = usize::from(parameter_count).div_ceil(8);

    if parameter_payload.len() < bitmap_len {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field: "null_bitmap",
            needed: bitmap_len,
            available: parameter_payload.len(),
        });
    }

    let mut null_parameter_indexes = Vec::new();
    for parameter_index in 0..usize::from(parameter_count) {
        let byte = parameter_payload[parameter_index / 8];
        let bit = parameter_index % 8;

        if byte & (1 << bit) != 0 {
            null_parameter_indexes.push(parameter_index);
        }
    }

    Ok(MysqlNullBitmap {
        null_parameter_indexes,
        bytes_consumed: bitmap_len,
    })
}

pub fn decode_numeric_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError> {
    decode_parameters(
        parameter_payload_after_null_bitmap,
        parameter_count,
        null_parameter_indexes,
    )
}

pub fn decode_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError> {
    if parameter_count == 0 {
        return Ok(Some(MysqlDecodedParameters {
            parameters: Vec::new(),
            bytes_consumed: 0,
        }));
    }

    let Some((&new_params_bind_flag, after_flag)) =
        parameter_payload_after_null_bitmap.split_first()
    else {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field: "new_params_bind_flag",
            needed: 1,
            available: 0,
        });
    };

    if new_params_bind_flag != MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT {
        return Ok(None);
    }

    let parameter_types = decode_parameter_types(after_flag, parameter_count)?;
    let mut remaining_values = &after_flag[usize::from(parameter_count) * 2..];
    let mut parameters = Vec::with_capacity(usize::from(parameter_count));

    for (parameter_index, parameter_type) in parameter_types.into_iter().enumerate() {
        let index = u16::try_from(parameter_index).expect("parameter index should fit u16");
        if null_parameter_indexes.contains(&parameter_index) {
            parameters.push(MysqlDecodedParameter {
                index,
                value: SqlParameterValue::Null,
            });
            continue;
        }

        let Some((value, consumed)) =
            decode_parameter_value(parameter_type, remaining_values, "parameter_value")?
        else {
            return Ok(None);
        };
        remaining_values = &remaining_values[consumed..];
        parameters.push(MysqlDecodedParameter { index, value });
    }

    let bytes_consumed = 1
        + usize::from(parameter_count) * 2
        + (after_flag[usize::from(parameter_count) * 2..].len() - remaining_values.len());

    Ok(Some(MysqlDecodedParameters {
        parameters,
        bytes_consumed,
    }))
}

fn decode_parameter_types(
    bytes: &[u8],
    parameter_count: u16,
) -> Result<Vec<MysqlParameterType>, MysqlExecuteParseError> {
    let needed = usize::from(parameter_count) * 2;
    if bytes.len() < needed {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field: "parameter_types",
            needed,
            available: bytes.len(),
        });
    }

    Ok(bytes[..needed]
        .chunks_exact(2)
        .map(|chunk| MysqlParameterType {
            type_code: chunk[0],
            unsigned: chunk[1] & MYSQL_PARAMETER_FLAG_UNSIGNED != 0,
        })
        .collect())
}

fn decode_parameter_value(
    parameter_type: MysqlParameterType,
    bytes: &[u8],
    field: &'static str,
) -> Result<Option<(SqlParameterValue, usize)>, MysqlExecuteParseError> {
    if let Some(decoded) = decode_numeric_parameter_value(parameter_type, bytes, field)? {
        return Ok(Some(decoded));
    }

    if let Some(decoded) = decode_temporal_parameter_value(parameter_type, bytes, field)? {
        return Ok(Some(decoded));
    }

    if is_text_type(parameter_type.type_code) {
        let (raw, consumed) = read_length_encoded_bytes(bytes, field)?;
        return Ok(Some((
            SqlParameterValue::String(String::from_utf8_lossy(raw).into_owned()),
            consumed,
        )));
    }

    if is_binary_summary_type(parameter_type.type_code) {
        let (raw, consumed) = read_length_encoded_bytes(bytes, field)?;
        return Ok(Some((
            SqlParameterValue::BinarySummary(binary_summary(raw)),
            consumed,
        )));
    }

    Ok(None)
}

fn decode_numeric_parameter_value(
    parameter_type: MysqlParameterType,
    bytes: &[u8],
    field: &'static str,
) -> Result<Option<(SqlParameterValue, usize)>, MysqlExecuteParseError> {
    let value = match parameter_type.type_code {
        MYSQL_TYPE_TINY => {
            let raw = read_value_bytes(bytes, 1, field)?;
            if parameter_type.unsigned {
                SqlParameterValue::Unsigned(u64::from(raw[0]))
            } else {
                SqlParameterValue::Integer(i64::from(i8::from_le_bytes([raw[0]])))
            }
        }
        MYSQL_TYPE_SHORT => {
            let raw = read_value_bytes(bytes, 2, field)?;
            let raw = [raw[0], raw[1]];
            if parameter_type.unsigned {
                SqlParameterValue::Unsigned(u64::from(u16::from_le_bytes(raw)))
            } else {
                SqlParameterValue::Integer(i64::from(i16::from_le_bytes(raw)))
            }
        }
        MYSQL_TYPE_LONG | MYSQL_TYPE_INT24 => {
            let raw = read_value_bytes(bytes, 4, field)?;
            let raw = [raw[0], raw[1], raw[2], raw[3]];
            if parameter_type.unsigned {
                SqlParameterValue::Unsigned(u64::from(u32::from_le_bytes(raw)))
            } else {
                SqlParameterValue::Integer(i64::from(i32::from_le_bytes(raw)))
            }
        }
        MYSQL_TYPE_LONGLONG => {
            let raw = read_value_bytes(bytes, 8, field)?;
            let raw = [
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ];
            if parameter_type.unsigned {
                SqlParameterValue::Unsigned(u64::from_le_bytes(raw))
            } else {
                SqlParameterValue::Integer(i64::from_le_bytes(raw))
            }
        }
        MYSQL_TYPE_FLOAT => {
            let raw = read_value_bytes(bytes, 4, field)?;
            let raw = [raw[0], raw[1], raw[2], raw[3]];
            SqlParameterValue::Float(f64::from(f32::from_le_bytes(raw)))
        }
        MYSQL_TYPE_DOUBLE => {
            let raw = read_value_bytes(bytes, 8, field)?;
            let raw = [
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ];
            SqlParameterValue::Float(f64::from_le_bytes(raw))
        }
        _ => return Ok(None),
    };

    Ok(Some((value, numeric_value_width(parameter_type.type_code))))
}

fn decode_temporal_parameter_value(
    parameter_type: MysqlParameterType,
    bytes: &[u8],
    field: &'static str,
) -> Result<Option<(SqlParameterValue, usize)>, MysqlExecuteParseError> {
    let Some(type_name) = temporal_type_name(parameter_type.type_code) else {
        return Ok(None);
    };

    let (raw, consumed) = read_length_encoded_bytes(bytes, field)?;
    let value = match parameter_type.type_code {
        MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE => {
            SqlParameterValue::Date(format_mysql_date(raw, field, type_name)?)
        }
        MYSQL_TYPE_TIME => SqlParameterValue::Time(format_mysql_time(raw, field, type_name)?),
        MYSQL_TYPE_DATETIME | MYSQL_TYPE_TIMESTAMP => {
            SqlParameterValue::Timestamp(format_mysql_datetime(raw, field, type_name)?)
        }
        _ => unreachable!("temporal_type_name only returns supported temporal type codes"),
    };

    Ok(Some((value, consumed)))
}

fn format_mysql_date(
    bytes: &[u8],
    field: &'static str,
    type_name: &'static str,
) -> Result<String, MysqlExecuteParseError> {
    match bytes.len() {
        0 => Ok("0000-00-00".to_owned()),
        4 => {
            let year = read_u16_le(bytes, 0);
            let month = bytes[2];
            let day = bytes[3];

            Ok(format!("{year:04}-{month:02}-{day:02}"))
        }
        length => Err(MysqlExecuteParseError::InvalidTemporalValueLength {
            field,
            type_name,
            length,
        }),
    }
}

fn format_mysql_datetime(
    bytes: &[u8],
    field: &'static str,
    type_name: &'static str,
) -> Result<String, MysqlExecuteParseError> {
    match bytes.len() {
        0 => Ok("0000-00-00 00:00:00".to_owned()),
        4 => Ok(format!(
            "{} 00:00:00",
            format_mysql_date(bytes, field, type_name)?
        )),
        7 | 11 => {
            let date = format_mysql_date(&bytes[..4], field, type_name)?;
            let hour = bytes[4];
            let minute = bytes[5];
            let second = bytes[6];
            let mut value = format!("{date} {hour:02}:{minute:02}:{second:02}");
            if bytes.len() == 11 {
                let micros = read_u32_le(bytes, 7);
                value.push_str(&format!(".{micros:06}"));
            }

            Ok(value)
        }
        length => Err(MysqlExecuteParseError::InvalidTemporalValueLength {
            field,
            type_name,
            length,
        }),
    }
}

fn format_mysql_time(
    bytes: &[u8],
    field: &'static str,
    type_name: &'static str,
) -> Result<String, MysqlExecuteParseError> {
    match bytes.len() {
        0 => Ok("00:00:00".to_owned()),
        8 | 12 => {
            let is_negative = bytes[0] != 0;
            let days = read_u32_le(bytes, 1);
            let hour = bytes[5];
            let minute = bytes[6];
            let second = bytes[7];
            let mut value = if days == 0 {
                format!("{hour:02}:{minute:02}:{second:02}")
            } else {
                format!("{days} {hour:02}:{minute:02}:{second:02}")
            };

            if bytes.len() == 12 {
                let micros = read_u32_le(bytes, 8);
                value.push_str(&format!(".{micros:06}"));
            }

            if is_negative {
                value.insert(0, '-');
            }

            Ok(value)
        }
        length => Err(MysqlExecuteParseError::InvalidTemporalValueLength {
            field,
            type_name,
            length,
        }),
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_length_encoded_bytes<'a>(
    bytes: &'a [u8],
    field: &'static str,
) -> Result<(&'a [u8], usize), MysqlExecuteParseError> {
    let (length, prefix_width) = read_length_encoded_integer(bytes, field)?;
    let length = usize::try_from(length)
        .map_err(|_| MysqlExecuteParseError::LengthEncodedValueTooLarge { field, length })?;
    let needed = prefix_width.checked_add(length).ok_or(
        MysqlExecuteParseError::LengthEncodedValueTooLarge {
            field,
            length: u64::try_from(length).unwrap_or(u64::MAX),
        },
    )?;
    let Some(raw) = bytes.get(prefix_width..needed) else {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field,
            needed,
            available: bytes.len(),
        });
    };

    Ok((raw, needed))
}

fn read_length_encoded_integer(
    bytes: &[u8],
    field: &'static str,
) -> Result<(u64, usize), MysqlExecuteParseError> {
    let Some((&marker, rest)) = bytes.split_first() else {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field,
            needed: 1,
            available: 0,
        });
    };

    match marker {
        0x00..=0xfa => Ok((u64::from(marker), 1)),
        MYSQL_LENGTH_ENCODED_NULL_MARKER => {
            Err(MysqlExecuteParseError::InvalidLengthEncodedInteger { field, marker })
        }
        MYSQL_LENGTH_ENCODED_U16_MARKER => {
            let raw = read_value_bytes(rest, 2, field)?;
            Ok((u64::from(u16::from_le_bytes([raw[0], raw[1]])), 3))
        }
        MYSQL_LENGTH_ENCODED_U24_MARKER => {
            let raw = read_value_bytes(rest, 3, field)?;
            let value = u32::from(raw[0]) | (u32::from(raw[1]) << 8) | (u32::from(raw[2]) << 16);
            Ok((u64::from(value), 4))
        }
        MYSQL_LENGTH_ENCODED_U64_MARKER => {
            let raw = read_value_bytes(rest, 8, field)?;
            Ok((
                u64::from_le_bytes([
                    raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                ]),
                9,
            ))
        }
        marker => Err(MysqlExecuteParseError::InvalidLengthEncodedInteger { field, marker }),
    }
}

fn read_value_bytes<'a>(
    bytes: &'a [u8],
    needed: usize,
    field: &'static str,
) -> Result<&'a [u8], MysqlExecuteParseError> {
    let Some(raw) = bytes.get(..needed) else {
        return Err(MysqlExecuteParseError::IncompletePayload {
            field,
            needed,
            available: bytes.len(),
        });
    };

    Ok(raw)
}

fn numeric_value_width(type_code: u8) -> usize {
    match type_code {
        MYSQL_TYPE_TINY => 1,
        MYSQL_TYPE_SHORT => 2,
        MYSQL_TYPE_LONG | MYSQL_TYPE_FLOAT | MYSQL_TYPE_INT24 => 4,
        MYSQL_TYPE_DOUBLE | MYSQL_TYPE_LONGLONG => 8,
        _ => 0,
    }
}

fn temporal_type_name(type_code: u8) -> Option<&'static str> {
    match type_code {
        MYSQL_TYPE_TIMESTAMP => Some("TIMESTAMP"),
        MYSQL_TYPE_DATE => Some("DATE"),
        MYSQL_TYPE_TIME => Some("TIME"),
        MYSQL_TYPE_DATETIME => Some("DATETIME"),
        MYSQL_TYPE_NEWDATE => Some("NEWDATE"),
        _ => None,
    }
}

fn is_text_type(type_code: u8) -> bool {
    matches!(
        type_code,
        MYSQL_TYPE_VARCHAR
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_STRING
            | MYSQL_TYPE_ENUM
            | MYSQL_TYPE_SET
    )
}

fn is_binary_summary_type(type_code: u8) -> bool {
    matches!(
        type_code,
        MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_BLOB
            | MYSQL_TYPE_BIT
            | MYSQL_TYPE_GEOMETRY
    )
}

fn binary_summary(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    let prefix_len = bytes.len().min(BINARY_SUMMARY_HEX_PREFIX_BYTES);
    let mut hex = String::with_capacity(prefix_len * 2);
    for byte in &bytes[..prefix_len] {
        hex.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }

    let suffix = if bytes.len() > prefix_len { "..." } else { "" };
    format!("len={} hex={hex}{suffix}", bytes.len())
}

fn render_parameter_literal(value: &SqlParameterValue) -> String {
    match value {
        SqlParameterValue::Null => "NULL".to_owned(),
        SqlParameterValue::Integer(value) => value.to_string(),
        SqlParameterValue::Unsigned(value) => value.to_string(),
        SqlParameterValue::Float(value) => value.to_string(),
        SqlParameterValue::Boolean(value) => {
            if *value {
                "TRUE".to_owned()
            } else {
                "FALSE".to_owned()
            }
        }
        SqlParameterValue::String(value)
        | SqlParameterValue::Date(value)
        | SqlParameterValue::Time(value)
        | SqlParameterValue::Timestamp(value)
        | SqlParameterValue::Json(value)
        | SqlParameterValue::BinarySummary(value)
        | SqlParameterValue::Unsupported(value) => quote_display_literal(value),
    }
}

fn quote_display_literal(value: &str) -> String {
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

fn starts_line_comment(bytes: &[u8], cursor: usize) -> bool {
    bytes.get(cursor..cursor + 3) == Some(b"-- ")
}

fn starts_block_comment(bytes: &[u8], cursor: usize) -> bool {
    bytes.get(cursor..cursor + 2) == Some(b"/*")
}

fn skip_quoted_sql(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if quote != b'`' && bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
            continue;
        }
        if bytes[cursor] == quote {
            if bytes.get(cursor + 1) == Some(&quote) {
                cursor += 2;
            } else {
                return cursor + 1;
            }
        } else {
            cursor += 1;
        }
    }

    bytes.len()
}

fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start;
    while cursor < bytes.len() {
        cursor += 1;
        if bytes[cursor - 1] == b'\n' {
            break;
        }
    }

    cursor
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 2;
    while cursor + 1 < bytes.len() {
        if bytes[cursor] == b'*' && bytes[cursor + 1] == b'/' {
            return cursor + 2;
        }
        cursor += 1;
    }

    bytes.len()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlExecuteParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
    InvalidLengthEncodedInteger {
        field: &'static str,
        marker: u8,
    },
    LengthEncodedValueTooLarge {
        field: &'static str,
        length: u64,
    },
    InvalidTemporalValueLength {
        field: &'static str,
        type_name: &'static str,
        length: usize,
    },
}

impl fmt::Display for MysqlExecuteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompletePayload {
                field,
                needed,
                available,
            } => write!(
                f,
                "incomplete MySQL COM_STMT_EXECUTE field `{field}`: needed {needed} bytes, available {available} bytes"
            ),
            Self::InvalidLengthEncodedInteger { field, marker } => write!(
                f,
                "invalid MySQL COM_STMT_EXECUTE length-encoded integer marker `{marker:#04x}` for field `{field}`"
            ),
            Self::LengthEncodedValueTooLarge { field, length } => write!(
                f,
                "MySQL COM_STMT_EXECUTE length-encoded value for field `{field}` is too large: {length} bytes"
            ),
            Self::InvalidTemporalValueLength {
                field,
                type_name,
                length,
            } => write!(
                f,
                "invalid MySQL COM_STMT_EXECUTE {type_name} value length for field `{field}`: {length} bytes"
            ),
        }
    }
}

impl Error for MysqlExecuteParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn length_encoded_value(bytes: &[u8]) -> Vec<u8> {
        let length = u8::try_from(bytes.len()).expect("test value should use one-byte length");
        let mut encoded = vec![length];
        encoded.extend_from_slice(bytes);

        encoded
    }

    fn mysql_date_value(year: u16, month: u8, day: u8) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend_from_slice(&year.to_le_bytes());
        value.push(month);
        value.push(day);

        length_encoded_value(&value)
    }

    fn mysql_datetime_value(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        micros: Option<u32>,
    ) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend_from_slice(&year.to_le_bytes());
        value.push(month);
        value.push(day);
        value.push(hour);
        value.push(minute);
        value.push(second);
        if let Some(micros) = micros {
            value.extend_from_slice(&micros.to_le_bytes());
        }

        length_encoded_value(&value)
    }

    fn mysql_time_value(
        is_negative: bool,
        days: u32,
        hour: u8,
        minute: u8,
        second: u8,
        micros: Option<u32>,
    ) -> Vec<u8> {
        let mut value = Vec::new();
        value.push(u8::from(is_negative));
        value.extend_from_slice(&days.to_le_bytes());
        value.push(hour);
        value.push(minute);
        value.push(second);
        if let Some(micros) = micros {
            value.extend_from_slice(&micros.to_le_bytes());
        }

        length_encoded_value(&value)
    }

    #[test]
    fn decodes_mixed_null_and_non_null_parameters() {
        let bitmap =
            decode_null_bitmap(&[0b1000_0101, 0b0000_0010], 10).expect("NULL bitmap should parse");

        assert_eq!(bitmap.null_parameter_indexes, [0, 2, 7, 9]);
        assert_eq!(bitmap.bytes_consumed, 2);
    }

    #[test]
    fn decodes_all_non_null_parameters() {
        let bitmap = decode_null_bitmap(&[0x00], 4).expect("all non-NULL bitmap should parse");

        assert!(bitmap.null_parameter_indexes.is_empty());
        assert_eq!(bitmap.bytes_consumed, 1);
    }

    #[test]
    fn decodes_zero_parameters() {
        let bitmap = decode_null_bitmap(&[], 0).expect("zero parameters should parse");

        assert!(bitmap.null_parameter_indexes.is_empty());
        assert_eq!(bitmap.bytes_consumed, 0);
    }

    #[test]
    fn ignores_padding_bits_beyond_parameter_count() {
        let bitmap = decode_null_bitmap(&[0xff, 0xff], 9).expect("NULL bitmap should parse");

        assert_eq!(bitmap.null_parameter_indexes, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(bitmap.bytes_consumed, 2);
    }

    #[test]
    fn rejects_truncated_null_bitmap() {
        let error = decode_null_bitmap(&[0x00], 9).expect_err("bitmap should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "null_bitmap",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn decodes_signed_integer_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_TINY,
            0x00,
            MYSQL_TYPE_SHORT,
            0x00,
            MYSQL_TYPE_LONG,
            0x00,
            MYSQL_TYPE_LONGLONG,
            0x00,
            MYSQL_TYPE_INT24,
            0x00,
        ]);
        payload.extend_from_slice(&i8::to_le_bytes(-1));
        payload.extend_from_slice(&i16::to_le_bytes(-2));
        payload.extend_from_slice(&i32::to_le_bytes(-3));
        payload.extend_from_slice(&i64::to_le_bytes(-4));
        payload.extend_from_slice(&i32::to_le_bytes(-5));

        let decoded = decode_numeric_parameters(&payload, 5, &[])
            .expect("numeric payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Integer(-1),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Integer(-2),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Integer(-3),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::Integer(-4),
                },
                MysqlDecodedParameter {
                    index: 4,
                    value: SqlParameterValue::Integer(-5),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_unsigned_integer_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_TINY,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
            MYSQL_TYPE_SHORT,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
            MYSQL_TYPE_LONG,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
            MYSQL_TYPE_LONGLONG,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
            MYSQL_TYPE_INT24,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
        ]);
        payload.extend_from_slice(&u8::to_le_bytes(250));
        payload.extend_from_slice(&u16::to_le_bytes(65_000));
        payload.extend_from_slice(&u32::to_le_bytes(4_000_000_000));
        payload.extend_from_slice(&u64::to_le_bytes(9_000_000_000));
        payload.extend_from_slice(&u32::to_le_bytes(16_000_000));

        let decoded = decode_numeric_parameters(&payload, 5, &[])
            .expect("numeric payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Unsigned(250),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Unsigned(65_000),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Unsigned(4_000_000_000),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::Unsigned(9_000_000_000),
                },
                MysqlDecodedParameter {
                    index: 4,
                    value: SqlParameterValue::Unsigned(16_000_000),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_float_and_double_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[MYSQL_TYPE_FLOAT, 0x00, MYSQL_TYPE_DOUBLE, 0x00]);
        payload.extend_from_slice(&f32::to_le_bytes(1.5));
        payload.extend_from_slice(&f64::to_le_bytes(2.25));

        let decoded = decode_numeric_parameters(&payload, 2, &[])
            .expect("numeric payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Float(1.5),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Float(2.25),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_null_numeric_parameter_without_consuming_value_bytes() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_LONG,
            0x00,
            MYSQL_TYPE_LONGLONG,
            MYSQL_PARAMETER_FLAG_UNSIGNED,
            MYSQL_TYPE_DOUBLE,
            0x00,
        ]);
        payload.extend_from_slice(&i32::to_le_bytes(-42));
        payload.extend_from_slice(&f64::to_le_bytes(2.5));

        let decoded = decode_numeric_parameters(&payload, 3, &[1])
            .expect("numeric payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Integer(-42),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Null,
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Float(2.5),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_text_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_VARCHAR,
            0x00,
            MYSQL_TYPE_VAR_STRING,
            0x00,
            MYSQL_TYPE_STRING,
            0x00,
            MYSQL_TYPE_ENUM,
            0x00,
            MYSQL_TYPE_SET,
            0x00,
        ]);
        payload.extend_from_slice(&length_encoded_value(b"alpha"));
        payload.extend_from_slice(&length_encoded_value(b"beta"));
        payload.extend_from_slice(&length_encoded_value(b"gamma"));
        payload.extend_from_slice(&length_encoded_value(b"one"));
        payload.extend_from_slice(&length_encoded_value(b"a,b"));

        let decoded = decode_parameters(&payload, 5, &[])
            .expect("text payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::String("alpha".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::String("beta".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::String("gamma".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::String("one".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 4,
                    value: SqlParameterValue::String("a,b".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn represents_invalid_text_without_panicking() {
        let payload = [
            MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
            MYSQL_TYPE_VARCHAR,
            0x00,
            0x03,
            0xff,
            b'a',
            0xfe,
        ];

        let decoded = decode_parameters(&payload, 1, &[])
            .expect("invalid text should be represented safely")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [MysqlDecodedParameter {
                index: 0,
                value: SqlParameterValue::String("\u{fffd}a\u{fffd}".to_owned()),
            }]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn summarizes_binary_parameters() {
        let long_binary: Vec<u8> = (0..20).collect();
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[MYSQL_TYPE_BLOB, 0x00, MYSQL_TYPE_GEOMETRY, 0x00]);
        payload.extend_from_slice(&length_encoded_value(&long_binary));
        payload.extend_from_slice(&length_encoded_value(&[0xab, 0xcd, 0xef]));

        let decoded = decode_parameters(&payload, 2, &[])
            .expect("binary payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::BinarySummary(
                        "len=20 hex=000102030405060708090a0b0c0d0e0f...".to_owned()
                    ),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::BinarySummary("len=3 hex=abcdef".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_mixed_numeric_text_binary_and_null_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_LONG,
            0x00,
            MYSQL_TYPE_VAR_STRING,
            0x00,
            MYSQL_TYPE_BLOB,
            0x00,
            MYSQL_TYPE_DOUBLE,
            0x00,
        ]);
        payload.extend_from_slice(&i32::to_le_bytes(7));
        payload.extend_from_slice(&length_encoded_value(b"ok"));
        payload.extend_from_slice(&length_encoded_value(&[0x00, 0x01]));

        let decoded = decode_parameters(&payload, 4, &[3])
            .expect("mixed payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Integer(7),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::String("ok".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::BinarySummary("len=2 hex=0001".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::Null,
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_date_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[MYSQL_TYPE_DATE, 0x00, MYSQL_TYPE_NEWDATE, 0x00]);
        payload.extend_from_slice(&mysql_date_value(2026, 7, 7));
        payload.extend_from_slice(&mysql_date_value(1999, 12, 31));

        let decoded = decode_parameters(&payload, 2, &[])
            .expect("date payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Date("2026-07-07".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Date("1999-12-31".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn represents_zero_length_temporal_values() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_DATE,
            0x00,
            MYSQL_TYPE_TIME,
            0x00,
            MYSQL_TYPE_DATETIME,
            0x00,
            MYSQL_TYPE_TIMESTAMP,
            0x00,
        ]);
        payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let decoded = decode_parameters(&payload, 4, &[])
            .expect("zero temporal values should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Date("0000-00-00".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Time("00:00:00".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Timestamp("0000-00-00 00:00:00".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::Timestamp("0000-00-00 00:00:00".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_time_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_TIME,
            0x00,
            MYSQL_TYPE_TIME,
            0x00,
            MYSQL_TYPE_TIME,
            0x00,
        ]);
        payload.extend_from_slice(&mysql_time_value(false, 0, 1, 2, 3, None));
        payload.extend_from_slice(&mysql_time_value(true, 2, 3, 4, 5, None));
        payload.extend_from_slice(&mysql_time_value(false, 0, 6, 7, 8, Some(901_234)));

        let decoded = decode_parameters(&payload, 3, &[])
            .expect("time payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Time("01:02:03".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Time("-2 03:04:05".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Time("06:07:08.901234".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn decodes_datetime_and_timestamp_parameters() {
        let mut payload = vec![MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT];
        payload.extend_from_slice(&[
            MYSQL_TYPE_DATETIME,
            0x00,
            MYSQL_TYPE_TIMESTAMP,
            0x00,
            MYSQL_TYPE_DATETIME,
            0x00,
        ]);
        payload.extend_from_slice(&mysql_datetime_value(2026, 7, 7, 9, 10, 11, None));
        payload.extend_from_slice(&mysql_datetime_value(
            2026,
            12,
            31,
            23,
            59,
            58,
            Some(123_456),
        ));
        payload.extend_from_slice(&mysql_date_value(2025, 1, 2));

        let decoded = decode_parameters(&payload, 3, &[])
            .expect("datetime payload should parse")
            .expect("type metadata should be present");

        assert_eq!(
            decoded.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Timestamp("2026-07-07 09:10:11".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Timestamp("2026-12-31 23:59:58.123456".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Timestamp("2025-01-02 00:00:00".to_owned()),
                },
            ]
        );
        assert_eq!(decoded.bytes_consumed, payload.len());
    }

    #[test]
    fn renders_expanded_sql_literals() {
        let parameters = [
            MysqlDecodedParameter {
                index: 0,
                value: SqlParameterValue::String("O'Reilly".to_owned()),
            },
            MysqlDecodedParameter {
                index: 1,
                value: SqlParameterValue::Null,
            },
            MysqlDecodedParameter {
                index: 2,
                value: SqlParameterValue::Integer(-42),
            },
            MysqlDecodedParameter {
                index: 3,
                value: SqlParameterValue::Unsigned(42),
            },
            MysqlDecodedParameter {
                index: 4,
                value: SqlParameterValue::Float(2.5),
            },
            MysqlDecodedParameter {
                index: 5,
                value: SqlParameterValue::Boolean(true),
            },
            MysqlDecodedParameter {
                index: 6,
                value: SqlParameterValue::Date("2026-07-07".to_owned()),
            },
            MysqlDecodedParameter {
                index: 7,
                value: SqlParameterValue::Time("01:02:03".to_owned()),
            },
            MysqlDecodedParameter {
                index: 8,
                value: SqlParameterValue::Timestamp("2026-07-07 09:10:11".to_owned()),
            },
            MysqlDecodedParameter {
                index: 9,
                value: SqlParameterValue::Json("{\"name\":\"sql-lens\"}".to_owned()),
            },
            MysqlDecodedParameter {
                index: 10,
                value: SqlParameterValue::BinarySummary("len=3 hex=abcdef".to_owned()),
            },
            MysqlDecodedParameter {
                index: 11,
                value: SqlParameterValue::Unsupported("unsupported".to_owned()),
            },
        ];

        let expanded =
            render_expanded_sql("select ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?", &parameters)
                .expect("expanded SQL should render");

        assert_eq!(
            expanded,
            "select 'O''Reilly', NULL, -42, 42, 2.5, TRUE, '2026-07-07', '01:02:03', '2026-07-07 09:10:11', '{\"name\":\"sql-lens\"}', 'len=3 hex=abcdef', 'unsupported'"
        );
    }

    #[test]
    fn renders_expanded_sql_skipping_non_placeholder_contexts() {
        let parameters = [
            MysqlDecodedParameter {
                index: 0,
                value: SqlParameterValue::Integer(1),
            },
            MysqlDecodedParameter {
                index: 1,
                value: SqlParameterValue::Integer(2),
            },
        ];

        let expanded = render_expanded_sql(
            "select '?' as single_q, \"?\" as double_q, `?` as ident, ? -- ? line\n, ? /* ? block */ # ? hash\n",
            &parameters,
        )
        .expect("expanded SQL should render");

        assert_eq!(
            expanded,
            "select '?' as single_q, \"?\" as double_q, `?` as ident, 1 -- ? line\n, 2 /* ? block */ # ? hash\n"
        );
    }

    #[test]
    fn renders_expanded_sql_skipping_escaped_quotes() {
        let parameters = [MysqlDecodedParameter {
            index: 0,
            value: SqlParameterValue::String("done".to_owned()),
        }];

        let expanded = render_expanded_sql(
            "select 'it''s ?' as doubled, 'escaped \\' ?' as backslash, ?",
            &parameters,
        )
        .expect("expanded SQL should render");

        assert_eq!(
            expanded,
            "select 'it''s ?' as doubled, 'escaped \\' ?' as backslash, 'done'"
        );
    }

    #[test]
    fn render_expanded_sql_rejects_missing_parameter() {
        let error = render_expanded_sql(
            "select ?, ?",
            &[MysqlDecodedParameter {
                index: 0,
                value: SqlParameterValue::Integer(1),
            }],
        )
        .expect_err("second placeholder should be missing");

        assert_eq!(
            error,
            MysqlExpandedSqlRenderError::MissingParameter {
                placeholder_index: 1,
                parameter_count: 1,
            }
        );
    }

    #[test]
    fn render_expanded_sql_rejects_extra_parameters() {
        let error = render_expanded_sql(
            "select ?",
            &[
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Integer(1),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Integer(2),
                },
            ],
        )
        .expect_err("extra parameter should fail");

        assert_eq!(
            error,
            MysqlExpandedSqlRenderError::ExtraParameters {
                placeholder_count: 1,
                parameter_count: 2,
            }
        );
    }

    #[test]
    fn returns_none_when_new_params_bind_flag_is_not_present() {
        let decoded =
            decode_numeric_parameters(&[0x00], 2, &[]).expect("flag should parse as unsupported");

        assert_eq!(decoded, None);
    }

    #[test]
    fn returns_none_for_unsupported_parameter_type() {
        let decoded = decode_parameters(&[MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT, 0xf6, 0x00], 1, &[])
            .expect("unsupported type should be non-fatal");

        assert_eq!(decoded, None);
    }

    #[test]
    fn handles_zero_numeric_parameters() {
        let decoded = decode_numeric_parameters(&[], 0, &[])
            .expect("zero parameters should parse")
            .expect("zero parameters should not need metadata");

        assert!(decoded.parameters.is_empty());
        assert_eq!(decoded.bytes_consumed, 0);
    }

    #[test]
    fn rejects_missing_new_params_bind_flag() {
        let error =
            decode_numeric_parameters(&[], 1, &[]).expect_err("bind flag should be missing");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "new_params_bind_flag",
                needed: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn rejects_truncated_parameter_type_metadata() {
        let error = decode_numeric_parameters(
            &[MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT, MYSQL_TYPE_LONG],
            1,
            &[],
        )
        .expect_err("parameter type metadata should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_types",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_truncated_numeric_value_bytes() {
        let error = decode_numeric_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_LONGLONG,
                0x00,
                0x01,
            ],
            1,
            &[],
        )
        .expect_err("numeric value should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_value",
                needed: 8,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_truncated_length_encoded_prefix() {
        let error = decode_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_VAR_STRING,
                0x00,
                MYSQL_LENGTH_ENCODED_U16_MARKER,
                0x01,
            ],
            1,
            &[],
        )
        .expect_err("length prefix should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_value",
                needed: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn rejects_truncated_text_value_bytes() {
        let error = decode_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_VAR_STRING,
                0x00,
                0x04,
                b'a',
                b'b',
            ],
            1,
            &[],
        )
        .expect_err("text value should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_value",
                needed: 5,
                available: 3,
            }
        );
    }

    #[test]
    fn rejects_truncated_binary_value_bytes() {
        let error = decode_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_BLOB,
                0x00,
                0x03,
                0xaa,
            ],
            1,
            &[],
        )
        .expect_err("binary value should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_value",
                needed: 4,
                available: 2,
            }
        );
    }

    #[test]
    fn rejects_unsupported_temporal_value_length() {
        let error = decode_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_DATE,
                0x00,
                0x01,
                0x00,
            ],
            1,
            &[],
        )
        .expect_err("temporal length should be unsupported");

        assert_eq!(
            error,
            MysqlExecuteParseError::InvalidTemporalValueLength {
                field: "parameter_value",
                type_name: "DATE",
                length: 1,
            }
        );
    }

    #[test]
    fn rejects_truncated_temporal_value_bytes() {
        let error = decode_parameters(
            &[
                MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT,
                MYSQL_TYPE_DATETIME,
                0x00,
                0x07,
                0xea,
                0x07,
                0x07,
            ],
            1,
            &[],
        )
        .expect_err("temporal value should be incomplete");

        assert_eq!(
            error,
            MysqlExecuteParseError::IncompletePayload {
                field: "parameter_value",
                needed: 8,
                available: 4,
            }
        );
    }

    #[test]
    fn parse_errors_have_display_messages() {
        assert_eq!(
            MysqlExecuteParseError::IncompletePayload {
                field: "null_bitmap",
                needed: 2,
                available: 1,
            }
            .to_string(),
            "incomplete MySQL COM_STMT_EXECUTE field `null_bitmap`: needed 2 bytes, available 1 bytes"
        );
    }
}
