use std::{
    io::{self, Write},
    process,
};

use execution::{execute, print_command_output};
use value::Value;

use crate::token::tokenize;
mod execution;
mod strings;
mod token;
mod value;

const REDIRECTIONS: [&str; 3] = [">", "1>", "2>"];

// Todo: implement colored prompt based on last exit code
fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read the input");
        let input = input.trim_end_matches(&['\n', '\r'][..]);

        let tokens = tokenize(input).unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        let (first, rest) = tokens.split_first().expect("Command not found!");
        let name = first.to_string();
        let raw_args = rest.to_vec();
        let args = Value::from_iter(raw_args.clone());

        // Todo: handle unknown command messages when strings are empty
        let redirection_found = raw_args
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()));

        let result = match redirection_found {
            Some(redirection_index) if redirection_index < raw_args.len() - 1 => {
                let redirection_type = &raw_args[redirection_index];
                let redirection_file = &raw_args[redirection_index + 1];
                if redirection_type == ">" || redirection_type == "1>" {
                    execute(
                        name,
                        args,
                        raw_args.clone(),
                        None,
                        Some(redirection_file.as_str()),
                        None,
                        false,
                        false,
                    )
                } else if redirection_type == "2>" {
                    execute(
                        name,
                        args,
                        raw_args.clone(),
                        None,
                        None,
                        Some(redirection_file.as_str()),
                        false,
                        false,
                    )
                } else {
                    todo!("Other redirection type will be implemented at a later time!");
                }
            }
            _ => execute(name, args, raw_args, None, None, None, false, false),
        };

        print_command_output(result);
    }
}
