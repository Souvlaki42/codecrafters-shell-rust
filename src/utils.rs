use std::io::{self, Write};

use crate::value::{FromValue, Value};

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush()?;
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

pub struct Arguments {
    _cmd: String,
    _args: Value,
}

impl Arguments {
    pub fn new(cmd: Vec<String>) -> Self {
        let (first, rest) = cmd.split_first().expect("Command not found!");
        let name = first.to_string();
        let args = Value::from_iter(rest.to_vec());

        Self {
            _cmd: name,
            _args: args,
        }
    }
    pub fn get<T: FromValue + Default>(&self, idx: usize, default: T) -> T {
        self._args.get(idx, default)
    }
    pub fn get_all(&self) -> Value {
        self._args.clone()
    }
    pub fn cmd(&self) -> String {
        self._cmd.clone()
    }
}
