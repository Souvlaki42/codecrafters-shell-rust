use std::{io::pipe, process, str::FromStr};

use rustyline::{config::BellStyle, CompletionType, Config, Editor};

use shell::{
    execution::{execute, get_external_executables, ExecuteArgs},
    lexer::tokenize,
    prompt::{get_input, Prompt},
    rw::RW,
};

use crate::shell::{execution::finalize_executions, lexer::RedirectionType};

mod shell;

fn main() {
    let (path_executables, path_keys) = get_external_executables();

    let prompt = Prompt::new(path_keys);

    let rl_config = Config::builder()
        .bell_style(BellStyle::Audible)
        .completion_type(CompletionType::List)
        .build();

    let mut rl = Editor::with_config(rl_config).expect("Failed to start the prompt!");

    rl.set_helper(Some(prompt));

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

        let mut exec_ouputs = Vec::new();

        if let Some(redirection_index) = tokens
            .iter()
            .position(|arg| RedirectionType::from_str(arg).is_ok())
            .filter(|&idx| idx < tokens.len() - 1)
        {
            let redirection_type = tokens[redirection_index].as_str();
            let path = &tokens[redirection_index + 1];

            let (append_output, append_error) = match redirection_type {
                ">>" | "1>>" => (true, false),
                "2>>" => (false, true),
                _ => (false, false),
            };

            match redirection_type {
                ">" | "1>" | ">>" | "1>>" => {
                    stdout = RW::File(path.to_string(), append_output);
                }
                "2>" | "2>>" => {
                    stderr = RW::File(path.to_string(), append_error);
                }
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
                eprintln!("Failed to create pipe: {}", e);
                process::exit(1);
            });
            let (pre_params, post_params) = params.split_at(pipe_index);

            let mut left_stdin = stdin;
            let mut left_stdout = RW::WPipe(Some(pipe_tx));
            let mut left_stderr = RW::Stderr;
            let left_exec = execute(ExecuteArgs {
                params: pre_params,
                path: &path_executables,
                stdin: &mut left_stdin,
                stdout: &mut left_stdout,
                stderr: &mut left_stderr,
            });

            let mut right_stdin = RW::RPipe(Some(pipe_rx));
            let mut right_stdout = RW::Stdout;
            let mut right_stderr = RW::Stderr;
            let right_exec = execute(ExecuteArgs {
                params: &post_params[1..],
                path: &path_executables,
                stdin: &mut right_stdin,
                stdout: &mut right_stdout,
                stderr: &mut right_stderr,
            });

            exec_ouputs.push(left_exec);
            exec_ouputs.push(right_exec);

            let final_output = finalize_executions(exec_ouputs);
            final_output.write_output(RW::Stdout, RW::Stderr);
            continue;
        }

        let output = execute(ExecuteArgs {
            params,
            path: &path_executables,
            stdin: &mut stdin,
            stdout: &mut stdout,
            stderr: &mut stderr,
        });

        exec_ouputs.push(output);

        let final_output = finalize_executions(exec_ouputs);
        final_output.write_output(stdout, stderr);
    }
}
