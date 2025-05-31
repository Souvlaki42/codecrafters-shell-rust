use itertools::Itertools;
use std::fmt;

pub type Integer = i32;
pub type Float = f32;

#[derive(Clone)]
pub enum Value {
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Vec<Value>),
    Anything(String),
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Anything(a) => write!(f, "{}", a),
            Value::Array(arr) => {
                write!(f, "{}", arr.iter().map(|k| k.to_string()).join(" "))
            }
        }
    }
}

impl Value {
    pub fn get<'a, T: FromValue<'a> + Default>(&'a self, index: usize, default: T) -> T {
        match self {
            Self::Array(vec) => vec
                .get(index)
                .and_then(|v| T::from_value(v))
                .unwrap_or(default),
            Self::Anything(s) if s.is_empty() => default,
            _ => T::from_value(self).unwrap_or(default),
        }
    }
}

/// Helper function to unescape strings
fn _unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn _parse_quoted_string(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];

    if (first == b'\'' || first == b'"') && first == last {
        let inner = &input[1..input.len() - 1];
        let quote_char = first as char;

        let mut result = String::new();
        let chars = inner.chars();
        let mut escaped = false;

        for c in chars {
            if escaped {
                // Accept any escaped character literally
                result.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote_char {
                // Unescaped quote of the same kind inside string is invalid
                return None;
            } else {
                result.push(c);
            }
        }

        if escaped {
            // Trailing backslash without escaped char is invalid
            return None;
        }

        Some(result)
    } else {
        None
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        // Try parse as Integer
        if let Ok(i) = value.parse::<Integer>() {
            return Value::Integer(i);
        }
        // Try parse as Float
        if let Ok(f) = value.parse::<Float>() {
            return Value::Float(f);
        }

        // Try parse as a fully quoted string
        if let Some(s) = _parse_quoted_string(&value) {
            return Value::String(s);
        }

        // If string contains quotes, parse mixed quoted/unquoted parts
        if value.contains('\'') || value.contains('"') {
            let mut result = String::new();
            let chars = value.chars().peekable();
            let mut current_part = String::new();
            let mut in_quote: Option<char> = None;
            let mut escaped = false;

            for c in chars {
                if escaped {
                    current_part.push(c);
                    escaped = false;
                    continue;
                }

                match c {
                    '\\' => {
                        escaped = true;
                    }
                    '\'' | '"' => {
                        if let Some(q) = in_quote {
                            if c == q {
                                // Closing quote: unescape current_part and append
                                let unescaped = _unescape_string(&current_part);
                                result.push_str(&unescaped);
                                current_part.clear();
                                in_quote = None;
                            } else {
                                // Different quote inside quote: keep as is
                                current_part.push(c);
                            }
                        } else {
                            // Opening quote
                            in_quote = Some(c);
                        }
                    }
                    _ => {
                        if in_quote.is_some() {
                            current_part.push(c);
                        } else {
                            result.push(c);
                        }
                    }
                }
            }

            // If still inside quote, invalid input; fallback to Anything
            if in_quote.is_some() || escaped {
                return Value::Anything(value);
            }

            // Append any trailing unquoted part
            if !current_part.is_empty() {
                result.push_str(&current_part);
            }

            return Value::String(result);
        }

        // Default fallback
        Value::Anything(value)
    }
}

impl FromIterator<String> for Value {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let input: Vec<String> = iter.into_iter().collect();

        if input.is_empty() {
            return Value::Anything(String::new());
        }

        if input.len() > 1 {
            let elements: Vec<Value> = input.into_iter().map(Value::from).collect();
            return Value::Array(elements);
        }

        Value::from(input[0].clone())
    }
}

pub trait FromValue<'a>: Sized {
    fn from_value(value: &'a Value) -> Option<Self>;
}

impl<'a> FromValue<'a> for Integer {
    fn from_value(value: &Value) -> Option<Self> {
        if let Value::Integer(i) = value {
            Some(*i)
        } else {
            None
        }
    }
}

impl<'a> FromValue<'a> for Float {
    fn from_value(value: &Value) -> Option<Self> {
        if let Value::Float(f) = value {
            Some(*f)
        } else {
            None
        }
    }
}

impl<'a> FromValue<'a> for &'a str {
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::String(s) => Some(s.as_str()),
            Value::Anything(s) => Some(s.as_str()),
            _ => None,
        }
    }
}
impl<'a> FromValue<'a> for String {
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::String(s) => Some(s.to_string()),
            Value::Anything(s) => Some(s.to_string()),
            _ => None,
        }
    }
}
