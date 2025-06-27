use itertools::Itertools;
use std::fmt;
use strum::{Display, EnumString};

use super::rw::RW;

pub type Integer = i32;
pub type Float = f32;
pub type Boolean = bool;

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString, Default)]
pub enum Redirection {
    #[default]
    #[strum(serialize = ">")]
    Default,
    #[strum(serialize = ">>")]
    DefaultAppend,
    #[strum(serialize = "1>")]
    Output,
    #[strum(serialize = "1>>")]
    OutputAppend,
    #[strum(serialize = "2>")]
    Error,
    #[strum(serialize = "2>>")]
    ErrorAppend,
}

#[derive(Debug)]
pub enum Value {
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Vec<Value>),
    Redirection(Box<Value>, Redirection, RW),
    Pipe(Box<Value>, Box<Value>),
    Anything(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "{}", s),
            Self::Anything(a) => write!(f, "{}", a),
            Self::Array(arr) => {
                write!(f, "{}", arr.iter().map(|k| k.to_string()).join(" "))
            }
            Self::Redirection(boxed, redirection, io) => {
                write!(f, "{} {} {:?}", *boxed, redirection, io)
            }
            Self::Pipe(pre_box, post_box) => {
                write!(f, "{} | {}", *pre_box, *post_box)
            }
        }
    }
}

impl Value {
    pub fn get<'a, T>(&'a self, index: usize, default: T) -> T
    where
        T: TryFrom<&'a Value> + Default,
    {
        match self {
            Self::Array(vec) => vec
                .get(index)
                .and_then(|v| T::try_from(v).ok())
                .unwrap_or(default),
            Self::Anything(s) if s.is_empty() => default,
            _ => T::try_from(self).ok().unwrap_or(default),
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

impl<'a> TryFrom<&'a Value> for Integer {
    type Error = anyhow::Error;
    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        if let Value::Integer(i) = value {
            Ok(*i)
        } else {
            anyhow::bail!("This value is not an integer!")
        }
    }
}

impl<'a> TryFrom<&'a Value> for Float {
    type Error = anyhow::Error;
    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        if let Value::Float(f) = value {
            Ok(*f)
        } else {
            anyhow::bail!("This value is not a float!")
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = anyhow::Error;
    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(s) => Ok(s.as_str()),
            Value::Anything(s) => Ok(s.as_str()),
            _ => anyhow::bail!("This value is not a string slice!"),
        }
    }
}

impl<'a> TryFrom<&'a Value> for String {
    type Error = anyhow::Error;
    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(s) => Ok(s.to_string()),
            Value::Anything(s) => Ok(s.to_string()),
            _ => anyhow::bail!("This value is not a string!"),
        }
    }
}
