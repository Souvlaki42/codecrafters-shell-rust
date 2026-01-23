use itertools::Itertools;
use std::{fmt, str::FromStr};

use super::strings::process_string;

pub type Integer = i32;
pub type Float = f32;
pub type Boolean = bool;

#[derive(Debug, Default)]
pub enum RedirectionType {
    #[default]
    Output,
    OutputAppend,
    Error,
    ErrorAppend,
}

impl FromStr for RedirectionType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" | "1>" => Ok(Self::Output),
            ">>" | "1>>" => Ok(Self::OutputAppend),
            "2>" => Ok(Self::Error),
            "2>>" => Ok(Self::ErrorAppend),
            _ => anyhow::bail!("Unknown redirection found! {s:}"),
        }
    }
}

#[derive(Debug)]
pub enum Token {
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Vec<Token>),
    Redirection(RedirectionType),
    Anything(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "{}", s),
            Self::Anything(a) => write!(f, "{}", a),
            Self::Redirection(redirection) => write!(f, "{:?}", redirection),
            Self::Array(arr) => {
                write!(f, "{}", arr.iter().map(|k| k.to_string()).join(" "))
            }
        }
    }
}

impl Token {
    pub fn get<'a, T>(&'a self, index: usize, default: T) -> T
    where
        T: TryFrom<&'a Self> + Default,
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

impl From<String> for Token {
    fn from(value: String) -> Self {
        // Try parse as Integer
        if let Ok(i) = value.parse::<Integer>() {
            return Self::Integer(i);
        }
        // Try parse as Float
        if let Ok(f) = value.parse::<Float>() {
            return Self::Float(f);
        }

        if let Ok(r) = RedirectionType::from_str(&value) {
            return Self::Redirection(r);
        }

        // Use the token as-is; tokenizer already processed it
        Self::String(value)
    }
}

impl FromIterator<String> for Token {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let input: Vec<String> = iter.into_iter().collect();

        if input.is_empty() {
            return Self::Anything(String::new());
        }

        if input.len() > 1 {
            let elements: Vec<Self> = input.into_iter().map(Self::from).collect();
            return Self::Array(elements);
        }

        Self::from(input[0].clone())
    }
}

impl<'a> TryFrom<&'a Token> for Integer {
    type Error = anyhow::Error;
    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        if let Token::Integer(i) = value {
            Ok(*i)
        } else {
            anyhow::bail!("This value is not an integer!")
        }
    }
}

impl<'a> TryFrom<&'a Token> for Float {
    type Error = anyhow::Error;
    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        if let Token::Float(f) = value {
            Ok(*f)
        } else {
            anyhow::bail!("This value is not a float!")
        }
    }
}

impl<'a> TryFrom<&'a Token> for &'a str {
    type Error = anyhow::Error;
    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        match value {
            Token::String(s) => Ok(s.as_str()),
            Token::Anything(s) => Ok(s.as_str()),
            _ => anyhow::bail!("This value is not a string slice!"),
        }
    }
}

impl<'a> TryFrom<&'a Token> for String {
    type Error = anyhow::Error;
    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        match value {
            Token::String(s) => Ok(s.to_string()),
            Token::Anything(s) => Ok(s.to_string()),
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
    tokens.iter().map(|t| process_string(t)).collect()
}
