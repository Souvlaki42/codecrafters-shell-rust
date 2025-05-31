use crate::strings;
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

        // Use the token as-is; tokenizer already processed it
        Value::String(value)
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
