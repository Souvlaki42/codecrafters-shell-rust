use std::process;

use execution::execute;
use io::IO;
use token::get_input_tokenized;
use value::Value;

mod execution;
mod io;
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
        let redirection_position: Option<usize> = raw_args
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()));

        match redirection_position {
            Some(position) if position + 1 < raw_args.len() => {
                let redirection_file = raw_args[position + 1].to_string();
                let mut out = IO::create_writer(Some(&redirection_file), false).unwrap();
                let mut err = IO::create_writer(Some(&redirection_file), true).unwrap();
                let redirection_type = raw_args[position].as_str();
                let raw_args = raw_args[..position].to_vec();
                let args = Value::from_iter(raw_args.clone());
                let result = execute(name, args, raw_args, (&out, &err));

                if redirection_type == "2>" {
                    result.to_error(&mut err);
                } else {
                    result.to_output(&mut out);
                }
            }
            _ => {
                let mut out = IO::create_writer(None, false).unwrap();
                let mut err = IO::create_writer(None, true).unwrap();
                let result = execute(name, args, raw_args, (&out, &err));
                result.to_output(&mut out);
                result.to_error(&mut err);
            }
        }
    }
}
