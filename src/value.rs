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

fn _parse_quoted_string(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];

    // Check if starts and ends with the same quote character (' or ")
    if (first == b'\'' || first == b'"') && first == last {
        // Extract inner content
        let inner = &input[1..input.len() - 1];

        // Reject strings containing unescaped quotes of the same kind
        // (simple check: no unescaped quote characters inside)
        let mut escaped = false;
        for c in inner.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
            } else if c == char::from(first) {
                // Found unescaped quote matching the delimiter
                return None;
            }
        }
        return Some(inner.to_string());
    }
    None
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        if let Ok(i) = value.parse::<Integer>() {
            return Value::Integer(i);
        }
        if let Ok(f) = value.parse::<Float>() {
            return Value::Float(f);
        }
        if let Some(s) = _parse_quoted_string(value.as_str()) {
            return Value::String(s);
        }

        Value::Anything(value.clone())
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
