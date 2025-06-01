use std::process;

use execution::{execute, print_command_output};
use itertools::Itertools;
use token::get_input_tokenized;
use value::Value;
mod execution;
mod strings;
mod token;
mod value;

const REDIRECTIONS: [&str; 3] = [">", "1>", "2>"];

// Todo: implement colored prompt based on last exit code
fn main() {
    loop {
        let tokens = get_input_tokenized().unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        let (first, rest) = tokens.split_first().expect("Command not found!");
        let name = first.to_string();
        let raw_args = rest.to_vec();
        let args = Value::from_iter(raw_args.clone());

        // Todo: handle unknown command messages when strings are empty
        let redirections_found: Vec<usize> = raw_args
            .iter()
            .positions(|arg| REDIRECTIONS.contains(&arg.as_str()))
            .collect();

        if redirections_found.is_empty() {
            let result = execute(name, args, raw_args);
            print_command_output(result);
        } else {
            todo!("Redirections are not implemented yet");
        }
    }
}
