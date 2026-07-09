pub fn fingerprint_sql(sql: &str) -> String {
    let mut scanner = FingerprintScanner::new(sql);
    scanner.scan();
    scanner.output
}

struct FingerprintScanner<'a> {
    input: &'a str,
    position: usize,
    output: String,
}

impl<'a> FingerprintScanner<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            output: String::new(),
        }
    }

    fn scan(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_whitespace() {
                self.consume_char();
                self.push_space();
            } else if ch == '\'' || ch == '"' {
                self.consume_quoted_literal(ch);
                self.push_placeholder();
            } else if ch.is_ascii_alphabetic() || ch == '_' {
                let word = self.consume_word();
                if is_placeholder_keyword(&word) {
                    self.push_placeholder();
                } else {
                    self.push_token(&word.to_ascii_lowercase());
                }
            } else if ch.is_ascii_digit() {
                self.consume_number_literal();
                self.push_placeholder();
            } else {
                self.consume_char();
                self.push_char(ch.to_ascii_lowercase());
            }
        }

        trim_trailing_space(&mut self.output);
        self.output = normalize_structural_spacing(&self.output);
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.input[self.position..].chars();
        chars.next()?;
        chars.next()
    }

    fn consume_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    fn consume_quoted_literal(&mut self, quote: char) {
        self.consume_char();

        while let Some(ch) = self.consume_char() {
            if ch == '\\' {
                self.consume_char();
            } else if ch == quote {
                if self.peek_char() == Some(quote) {
                    self.consume_char();
                } else {
                    break;
                }
            }
        }
    }

    fn consume_word(&mut self) -> String {
        let start = self.position;

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
                self.consume_char();
            } else {
                break;
            }
        }

        self.input[start..self.position].to_owned()
    }

    fn consume_number_literal(&mut self) {
        if self.peek_char() == Some('0') && matches!(self.peek_next_char(), Some('x' | 'X')) {
            self.consume_char();
            self.consume_char();
            self.consume_while(|ch| ch.is_ascii_hexdigit());
            return;
        }

        self.consume_while(|ch| ch.is_ascii_digit());

        if self.peek_char() == Some('.')
            && self.peek_next_char().is_some_and(|ch| ch.is_ascii_digit())
        {
            self.consume_char();
            self.consume_while(|ch| ch.is_ascii_digit());
        }

        if matches!(self.peek_char(), Some('e' | 'E')) {
            let exponent_start = self.position;
            self.consume_char();
            if matches!(self.peek_char(), Some('+' | '-')) {
                self.consume_char();
            }
            let digit_start = self.position;
            self.consume_while(|ch| ch.is_ascii_digit());

            if self.position == digit_start {
                self.position = exponent_start;
            }
        }
    }

    fn consume_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while let Some(ch) = self.peek_char() {
            if predicate(ch) {
                self.consume_char();
            } else {
                break;
            }
        }
    }

    fn push_space(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
    }

    fn push_placeholder(&mut self) {
        self.push_token("?");
    }

    fn push_token(&mut self, token: &str) {
        self.output.push_str(token);
    }

    fn push_char(&mut self, ch: char) {
        self.output.push(ch);
    }
}

fn is_placeholder_keyword(word: &str) -> bool {
    word.eq_ignore_ascii_case("null")
        || word.eq_ignore_ascii_case("true")
        || word.eq_ignore_ascii_case("false")
}

fn trim_trailing_space(value: &mut String) {
    while value.ends_with(' ') {
        value.pop();
    }
}

fn normalize_structural_spacing(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut normalized = String::with_capacity(value.len());
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if ch == ' ' {
            let previous = previous_non_space(&normalized);
            let next = next_non_space(&chars, index + 1);

            if previous == Some('(')
                || previous == Some(',')
                || previous.is_some_and(is_compact_operator)
                || next.is_some_and(|next| {
                    matches!(next, ',' | ')' | ';') || is_compact_operator(next)
                })
            {
                index += 1;
                continue;
            }
        } else if matches!(ch, ',' | ')' | ';') || is_compact_operator(ch) {
            trim_trailing_space(&mut normalized);
        }

        normalized.push(ch);
        index += 1;
    }

    trim_trailing_space(&mut normalized);
    normalized
}

fn previous_non_space(value: &str) -> Option<char> {
    value.chars().rev().find(|ch| *ch != ' ')
}

fn next_non_space(chars: &[char], start: usize) -> Option<char> {
    chars[start..].iter().copied().find(|ch| *ch != ' ')
}

fn is_compact_operator(ch: char) -> bool {
    matches!(ch, '=' | '<' | '>' | '!')
}

#[cfg(test)]
mod tests {
    use super::fingerprint_sql;

    #[test]
    fn fingerprints_select_literals_and_whitespace() {
        let sql = " SELECT  *\nFROM users\tWHERE id = 42 AND active = TRUE ";

        assert_eq!(
            fingerprint_sql(sql),
            "select * from users where id=? and active=?"
        );
    }

    #[test]
    fn fingerprints_insert_literals() {
        let sql = "INSERT INTO users (id, name, deleted_at) VALUES (123, 'Ada', NULL)";

        assert_eq!(
            fingerprint_sql(sql),
            "insert into users (id,name,deleted_at) values (?,?,?)"
        );
    }

    #[test]
    fn fingerprints_update_literals() {
        let sql = r#"UPDATE users SET name = "Grace", score = 98.5 WHERE id = 7"#;

        assert_eq!(
            fingerprint_sql(sql),
            "update users set name=?,score=? where id=?"
        );
    }

    #[test]
    fn fingerprints_delete_literals() {
        let sql = "DELETE FROM sessions WHERE token = 'abc' AND user_id = 0x2A";

        assert_eq!(
            fingerprint_sql(sql),
            "delete from sessions where token=? and user_id=?"
        );
    }

    #[test]
    fn preserves_digits_inside_identifiers() {
        let sql = "SELECT col1 FROM table2 WHERE id = 1";

        assert_eq!(fingerprint_sql(sql), "select col1 from table2 where id=?");
    }

    #[test]
    fn handles_escaped_quotes_best_effort() {
        let sql = r#"SELECT 'it\'s ok', 'it''s also ok'"#;

        assert_eq!(fingerprint_sql(sql), "select ?,?");
    }

    #[test]
    fn handles_unclosed_string_literals_best_effort() {
        let sql = "SELECT 'unterminated";

        assert_eq!(fingerprint_sql(sql), "select ?");
    }

    #[test]
    fn normalizes_spacing_around_punctuation_and_comparison_operators() {
        let left = "SELECT * FROM users WHERE id=42 AND name='Ada'";
        let right = " SELECT *  FROM users WHERE id = 42 AND name = 'Ada' ";

        assert_eq!(fingerprint_sql(left), fingerprint_sql(right));
        assert_eq!(
            fingerprint_sql(left),
            "select * from users where id=? and name=?"
        );
    }
}
