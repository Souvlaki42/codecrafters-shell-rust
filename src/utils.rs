use std::{
    io::{self, Write},
    process::Command,
};

use anyhow::{Context, Ok};

use crate::value::{FromValue, Integer, Value};

// Todo: add clear builtin
pub const BUILTINS: [&str; 5] = ["echo", "type", "exit", "pwd", "cd"];

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut inside_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for c in input.trim().chars() {
        if escaped {
            current_token.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
            }
            '\'' | '"' => {
                if !inside_quotes {
                    inside_quotes = true;
                    quote_char = c;
                } else if c == quote_char {
                    inside_quotes = false;
                } else {
                    current_token.push(c);
                }
            }
            ' ' => {
                if !inside_quotes {
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                } else {
                    current_token.push(c);
                }
            }
            _ => {
                current_token.push(c);
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    Ok(tokens)
}

pub fn execute_external(cmd: &str, args: Vec<String>) -> anyhow::Result<(String, String, Integer)> {
    let process = Command::new(cmd)
        .args(args)
        .spawn()
        .context("Running process error")?;

    let output = process
        .wait_with_output()
        .context("Retrieving output error")?;

    let stdout = String::from_utf8(output.stdout).context("Translating stdout error")?;
    let stderr = String::from_utf8(output.stderr).context("Translating stderr error")?;
    let status = output.status.code().unwrap_or_default();

    Ok((stdout, stderr, status))
}

pub struct Arguments {
    _cmd: String,
    _args: Value,
    _raw_args: Vec<String>,
}

impl Arguments {
    pub fn new(cmd: Vec<String>) -> Self {
        let (first, rest) = cmd.split_first().expect("Command not found!");
        let name = first.to_string();
        let _raw_args = rest.to_vec();
        let args = Value::from_iter(rest.to_vec());

        Self {
            _cmd: name,
            _args: args,
            _raw_args,
        }
    }
    pub fn get<'a, T: FromValue<'a> + Default>(&'a self, idx: usize, default: T) -> T {
        self._args.get(idx, default)
    }
    pub fn get_all(&self) -> Value {
        self._args.clone()
    }
    pub fn get_raw(&self) -> Vec<String> {
        self._raw_args.clone()
    }
    pub fn cmd(&self) -> String {
        self._cmd.clone()
    }
}
