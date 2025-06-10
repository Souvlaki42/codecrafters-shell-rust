use std::process;

use execution::execute;
use rustyline::{config::BellStyle, CompletionType, Config, Editor};

use crate::{
    execution::{get_external_executables, ExecuteArgs},
    input::{get_input, tokenize, Shell},
};
mod execution;
mod input;
mod strings;
mod value;

const REDIRECTIONS: [&str; 6] = [">", "1>", "2>", ">>", "1>>", "2>>"];

// Todo: implement colored prompt based on last exit code
fn main() {
    let path_executables = get_external_executables();
    let path_keys: Vec<String> = path_executables.keys().map(|k| k.to_string()).collect();
    let shell = Shell::new(path_keys);

    let rl_config = Config::builder()
        .bell_style(BellStyle::Audible)
        .completion_type(CompletionType::List)
        .build();
    let mut rl = Editor::with_config(rl_config).expect("Failed to start the prompt!");
    rl.set_helper(Some(shell));

    loop {
        let input = get_input(&mut rl, "$ ");

        if input.is_none() {
            continue;
        }

        let tokens = tokenize(&input.unwrap()).unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        let mut stdout_path: Option<&str> = None;
        let mut stderr_path: Option<&str> = None;

        let mut append_output = false;
        let mut append_error = false;

        let mut params: &[String] = &tokens;

        if let Some(redirection_index) = tokens
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()))
            .filter(|&idx| idx < tokens.len() - 1)
        {
            let redirection_type = tokens[redirection_index].as_str();
            let path = tokens[redirection_index + 1].as_str();

            (append_output, append_error) = match redirection_type {
                ">>" | "1>>" => (true, false),
                "2>>" => (false, true),
                _ => (false, false),
            };

            match redirection_type {
                ">" | "1>" | ">>" | "1>>" => stdout_path = Some(path),
                "2>" | "2>>" => stderr_path = Some(path),
                _ => todo!("Other redirection types"),
            }

            params = &tokens[..redirection_index];
        }

        let args = ExecuteArgs {
            params,
            path: &path_executables,
            input_file: None,
            output_file: stdout_path,
            error_file: stderr_path,
            append_output,
            append_error,
        };
        let result = execute(args);

        result.send_output(stdout_path, stderr_path, append_output, append_error);
    }
}
