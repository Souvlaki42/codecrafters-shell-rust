use itertools::Itertools;
use std::fmt;

use super::strings::process_string;

pub type Integer = i32;
pub type Float = f32;
pub type Boolean = bool;

pub const REDIRECTIONS: [&str; 6] = [">", "1>", "2>", ">>", "1>>", "2>>"];

#[derive(Debug)]
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
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "{}", s),
            Self::Anything(a) => write!(f, "{}", a),
            Self::Array(arr) => {
                write!(f, "{}", arr.iter().map(|k| k.to_string()).join(" "))
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

pub fn tokenize(input: &str) -> anyhow::Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let chars = input.chars().peekable();

    let mut in_quote: Option<char> = None;
    let mut escaped = false;

    for c in chars {
        if escaped {
            // Previous char was backslash, so push both backslash and this char literally
            current_token.push('\\');
            current_token.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
            }
            '\'' | '"' if in_quote.is_none() => {
                in_quote = Some(c);
                current_token.push(c);
            }
            c if in_quote == Some(c) => {
                in_quote = None;
                current_token.push(c);
            }
            ' ' | '\t' if in_quote.is_none() => {
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
            }
            _ => {
                current_token.push(c);
            }
        }
    }

    if escaped {
        anyhow::bail!("Trailing escape character");
    }

    if in_quote.is_some() {
        anyhow::bail!("Unclosed quote in input");
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    // Now process tokens with strings::process_string to handle quoting and unescaping
    tokens
        .into_iter()
        .map(|token| process_string(&token))
        .collect()
}
