use std::process;

use execution::execute;
use rustyline::{config::BellStyle, CompletionType, Config, Editor};

use crate::{
    execution::{get_external_executables, ExecuteArgs},
    input::{get_input, tokenize, Shell},
    io::IO,
};

mod execution;
mod input;
mod io;
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

        let mut stdin = IO::Stdin;
        let mut stdout = IO::Stdout;
        let mut stderr = IO::Stderr;

        let mut params: &[String] = &tokens;

        if let Some(redirection_index) = tokens
            .iter()
            .position(|arg| REDIRECTIONS.contains(&arg.as_str()))
            .filter(|&idx| idx < tokens.len() - 1)
        {
            let redirection_type = tokens[redirection_index].as_str();
            let path = &tokens[redirection_index + 1];

            let (append_output, append_error) = match redirection_type {
                ">>" | "1>>" => (true, false),
                "2>>" => (false, true),
                _ => (false, false),
            };

            (stdout, stderr) = match redirection_type {
                ">" | "1>" | ">>" | "1>>" => (IO::File(path.to_string(), append_output), IO::Null),
                "2>" | "2>>" => (IO::File(path.to_string(), append_error), IO::Null),
                _ => todo!("Other redirection types"),
            };

            params = &tokens[..redirection_index];
        }

        let args = ExecuteArgs {
            params,
            path: &path_executables,
            stdin: &mut stdin,
            stdout: &mut stdout,
            stderr: &mut stderr,
        };
        let result = execute(args);

        result.send_output(stdout, stderr);
    }
}
