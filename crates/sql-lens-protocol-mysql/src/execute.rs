use std::{error::Error, fmt};

use sql_lens_core::SqlParameterValue;

const MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT: u8 = 0x01;
const MYSQL_PARAMETER_FLAG_UNSIGNED: u8 = 0x80;

const MYSQL_TYPE_TINY: u8 = 0x01;
const MYSQL_TYPE_SHORT: u8 = 0x02;
const MYSQL_TYPE_LONG: u8 = 0x03;
const MYSQL_TYPE_FLOAT: u8 = 0x04;
const MYSQL_TYPE_DOUBLE: u8 = 0x05;
const MYSQL_TYPE_LONGLONG: u8 = 0x08;
const MYSQL_TYPE_INT24: u8 = 0x09;

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
            decode_numeric_parameter_value(parameter_type, remaining_values, "parameter_value")?
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlExecuteParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
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
        }
    }
}

impl Error for MysqlExecuteParseError {}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn returns_none_when_new_params_bind_flag_is_not_present() {
        let decoded =
            decode_numeric_parameters(&[0x00], 2, &[]).expect("flag should parse as unsupported");

        assert_eq!(decoded, None);
    }

    #[test]
    fn returns_none_for_unsupported_parameter_type() {
        let decoded =
            decode_numeric_parameters(&[MYSQL_NEW_PARAMS_BOUND_FLAG_PRESENT, 0xfd, 0x00], 1, &[])
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
