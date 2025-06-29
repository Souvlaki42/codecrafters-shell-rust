use std::{io::pipe, process};

use rustyline::{config::BellStyle, CompletionType, Config, Editor};

use shell::{
    execution::{execute, get_external_executables, ExecuteArgs},
    prompt::{get_input, Prompt},
    rw::RW,
    value::tokenize,
};

use crate::shell::{execution::finalize_executions, value::REDIRECTIONS};

mod shell;

// Todo: implement colored prompt based on last exit code
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
                eprintln!("Faled to create pipe: {}", e);
                process::exit(1);
            });
            let (pipe_in, mut pipe_out) = (RW::RPipe(Some(pipe_rx)), RW::WPipe(Some(pipe_tx)));

            let (pre_params, post_params) = params.split_at(pipe_index);

            let output = execute(ExecuteArgs {
                params: pre_params,
                path: &path_executables,
                stdin: &mut stdin,
                stdout: &mut pipe_out,
                stderr: &mut stderr,
            });

            exec_ouputs.push(output);

            stdin = pipe_in;
            params = &post_params[1..];
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
