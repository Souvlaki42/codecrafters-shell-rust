use std::{
    io::{self, Write},
    process,
};

use execution::execute;

use crate::token::tokenize;
mod execution;
mod strings;
mod token;
mod value;

const REDIRECTIONS: [&str; 6] = [">", "1>", "2>", ">>", "1>>", "2>>"];

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
        let args = rest.to_vec();

        let mut stdout_path: Option<&str> = None;
        let mut stderr_path: Option<&str> = None;

        let mut append_output = false;
        let mut append_error = false;

        let result = if let Some(redirection_index) = args
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()))
        {
            let redirection_type = &args[redirection_index].as_str();
            let path = args.get(redirection_index + 1).map(|s| s.as_str());

            append_output = *redirection_type == ">>" || *redirection_type == "1>>";
            append_error = *redirection_type == "2>>";

            match *redirection_type {
                ">" | "1>" | ">>" | "1>>" => stdout_path = path,
                "2>" | "2>>" => stderr_path = path,
                _ => todo!("Other redirection types"),
            }
            // Then execute with these paths:
            execute(
                name,
                &args[..redirection_index],
                None,
                stdout_path,
                stderr_path,
                append_output,
                append_error,
            )
        } else {
            execute(
                name,
                args.as_slice(),
                None,
                None,
                None,
                append_output,
                append_error,
            )
        };

        result.send_output(stdout_path, stderr_path, append_output, append_error);
    }
}
