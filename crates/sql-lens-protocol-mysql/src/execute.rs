use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlNullBitmap {
    pub null_parameter_indexes: Vec<usize>,
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
