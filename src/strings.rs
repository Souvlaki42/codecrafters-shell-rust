/// Represents the state of string processing
#[derive(Debug)]
struct StringState {
    result: String,
    current_part: String,
    in_quote: Option<char>,
}

impl StringState {
    fn new() -> Self {
        Self {
            result: String::new(),
            current_part: String::new(),
            in_quote: None,
        }
    }

    fn finish_quote(&mut self) {
        match self.in_quote {
            Some('\'') => {
                // Single quotes: everything is literal
                self.result.push_str(&self.current_part);
            }
            Some('"') => {
                // Double quotes: unescape the content
                self.result.push_str(&unescape_string(&self.current_part));
            }
            _ => unreachable!(),
        }
        self.current_part.clear();
        self.in_quote = None;
    }

    fn handle_backslash(&mut self, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        match self.in_quote {
            Some(quote) => match quote {
                '\'' => {
                    // Inside single quotes, backslash is literal
                    self.current_part.push('\\');
                    if let Some(next) = chars.next() {
                        self.current_part.push(next);
                    }
                }
                '"' => {
                    // Inside double quotes, only escape " and \
                    if let Some(&next) = chars.peek() {
                        if next == '"' || next == '\\' {
                            chars.next(); // consume escaped char
                            self.current_part.push(next);
                        } else {
                            self.current_part.push('\\');
                        }
                    } else {
                        self.current_part.push('\\');
                    }
                }
                _ => {
                    // For any other quote type (shouldn't happen), treat as literal
                    self.current_part.push('\\');
                }
            },
            None => {
                // Outside quotes, escape next character
                if let Some(next) = chars.next() {
                    self.result.push(next);
                } else {
                    self.result.push('\\');
                }
            }
        }
    }

    fn finish(&mut self) -> anyhow::Result<String> {
        if !self.current_part.is_empty() {
            if let Some(quote) = self.in_quote {
                match quote {
                    '\'' => self.result.push_str(&self.current_part),
                    '"' => self.result.push_str(&unescape_string(&self.current_part)),
                    _ => unreachable!(),
                }
            } else {
                self.result.push_str(&self.current_part);
            }
        }

        if self.in_quote.is_some() {
            anyhow::bail!("Unclosed quote in input");
        }

        Ok(self.result.clone())
    }
}

/// Processes a string according to shell rules:
/// - Single quotes ('): Everything inside is literal
/// - Double quotes ("): Allows escaping of " and \ with backslash
/// - Outside quotes: Backslash escapes next character
pub fn process_string(input: &str) -> anyhow::Result<String> {
    let mut state = StringState::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' if state.in_quote.is_none() => {
                // Start of quote
                state.in_quote = Some(c);
            }
            c if state.in_quote == Some(c) => {
                // End of quote
                state.finish_quote();
            }
            '\\' => {
                state.handle_backslash(&mut chars);
            }
            _ => {
                if state.in_quote.is_some() {
                    state.current_part.push(c);
                } else {
                    state.result.push(c);
                }
            }
        }
    }

    state.finish()
}

/// Unescapes a string according to double-quote rules:
/// - \" becomes "
/// - \\ becomes \
/// - Other backslashes are preserved
fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == '"' || next == '\\' {
                    chars.next(); // consume escaped char
                    result.push(next);
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_quotes() {
        assert_eq!(process_string("'hello\\'world'").unwrap(), "hello\\'world");
        assert_eq!(process_string("'hello\"world'").unwrap(), "hello\"world");
        assert_eq!(process_string("'hello\\world'").unwrap(), "hello\\world");
    }

    #[test]
    fn test_double_quotes() {
        assert_eq!(
            process_string("\"hello\\\"world\"").unwrap(),
            "hello\"world"
        );
        assert_eq!(
            process_string("\"hello\\'world\"").unwrap(),
            "hello\\'world"
        );
        assert_eq!(process_string("\"hello\\world\"").unwrap(), "hello\\world");
    }

    #[test]
    fn test_mixed_quotes() {
        assert_eq!(process_string("'hello'\"world\"").unwrap(), "helloworld");
        assert_eq!(process_string("\"hello\"'world'").unwrap(), "helloworld");
    }

    #[test]
    fn test_unclosed_quotes() {
        assert!(process_string("'hello").is_err());
        assert!(process_string("\"hello").is_err());
    }

    #[test]
    fn test_backslash_escaping() {
        assert_eq!(process_string("hello\\ world").unwrap(), "hello world");
        assert_eq!(process_string("hello\\\\world").unwrap(), "hello\\world");
        assert_eq!(process_string("hello\\nworld").unwrap(), "hellonworld");
    }

    #[test]
    fn test_complex_quotes() {
        assert_eq!(
            process_string("'script\\\"worldtest\\\"shell'").unwrap(),
            "script\\\"worldtest\\\"shell"
        );
        assert_eq!(
            process_string("\"script\\\"worldtest\\\"shell\"").unwrap(),
            "script\"worldtest\"shell"
        );
    }
}
