use std::{io::pipe, process};

use rustyline::{config::BellStyle, CompletionType, Config, Editor};

use shell::{
    execution::{execute, get_external_executables, ExecuteArgs},
    input::{get_input, tokenize, Shell},
    rw::RW,
};

use crate::shell::value::Redirection;

mod shell;

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

        let mut stdin = RW::Stdin;
        let mut stdout = RW::Stdout;
        let mut stderr = RW::Stderr;

        let mut params: &[String] = &tokens;

        if let Some(redirection_index) = tokens
            .iter()
            .position(|arg| arg.parse::<Redirection>().is_ok())
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
                ">" | "1>" | ">>" | "1>>" => (RW::File(path.to_string(), append_output), RW::Null),
                "2>" | "2>>" => (RW::File(path.to_string(), append_error), RW::Null),
                _ => todo!("Other redirection types"),
            };

            params = &tokens[..redirection_index];
        }

        if let Some(pipe_index) = tokens
            .iter()
            .position(|arg| arg == "|")
            .filter(|&idx| idx < tokens.len() - 1)
        {
            let (pipe_rx, pipe_tx) = pipe().unwrap_or_else(|e| {
                eprintln!("Faled to create pipe: {}", e);
                process::exit(1);
            });
            let (pipe_in, mut pipe_out) = (RW::RPipe(Some(pipe_rx)), RW::WPipe(Some(pipe_tx)));

            let (pre_params, post_params) = params.split_at(pipe_index);

            execute(ExecuteArgs {
                params: pre_params,
                path: &path_executables,
                stdin: &mut stdin,
                stdout: &mut pipe_out,
                stderr: &mut stderr,
            });

            stdin = pipe_in;
            params = &post_params[1..];
        }

        let result = execute(ExecuteArgs {
            params,
            path: &path_executables,
            stdin: &mut stdin,
            stdout: &mut stdout,
            stderr: &mut stderr,
        });

        result.send_output(stdout, stderr);
    }
}
