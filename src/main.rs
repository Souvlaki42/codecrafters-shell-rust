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
        let args = rest.to_vec();

        let mut stdout_path: Option<&str> = None;
        let mut stderr_path: Option<&str> = None;

        let result = if let Some(redirection_index) = args
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()))
        {
            let redirection_type = &args[redirection_index];
            let path = args.get(redirection_index + 1).map(|s| s.as_str());

            match redirection_type.as_str() {
                ">" | "1>" => stdout_path = path,
                "2>" => stderr_path = path,
                _ => todo!("Other redirection types"),
            }
            // Then execute with these paths:
            execute(
                name,
                &args[..redirection_index],
                None,
                stdout_path,
                stderr_path,
                false,
                false,
            )
        } else {
            execute(name, args.as_slice(), None, None, None, false, false)
        };

        result.send_output(stdout_path, stderr_path);
    }
}
