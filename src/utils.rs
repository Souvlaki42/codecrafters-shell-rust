use std::{
    io::{self, Write},
    process::Command,
};

use anyhow::{Context, Ok};

use crate::value::{FromValue, Integer, Value};

pub const BUILTINS: [&str; 3] = ["echo", "type", "exit"];

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mut inside_string = false;
    Ok(input
        .trim()
        .split(|c| {
            if c == '\'' || c == '"' {
                inside_string = !inside_string;
            }
            !inside_string && c == ' '
        })
        .map(|token| token.to_string())
        .collect())
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
    pub fn get<T: FromValue + Default>(&self, idx: usize, default: T) -> T {
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
