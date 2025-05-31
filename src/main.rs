#[cfg(test)]
mod tests;

use anyhow::Context;
use std::{
    env,
    io::{self, ErrorKind, Write},
    path::Path,
    process::{self, Command},
};
use token::get_input_tokenized;
use value::{Integer, Value};
use which::which;

mod strings;
mod token;
mod value;

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "clear"];

fn execute_external(cmd: &str, args: &Vec<String>) -> anyhow::Result<(String, String, Integer)> {
    let process = Command::new(cmd)
        .args(args)
        .spawn()
        .context("Running process error")?;

    let output = process
        .wait_with_output()
        .context("Retrieving output error")?;

    let stdout = String::from_utf8(output.stdout).context("Translating stdout error")?;
    let stderr = String::from_utf8(output.stderr).context("Translating stderr error")?;
    let status = output.status.code().unwrap_or_default();

    Ok((stdout, stderr, status))
}

fn main() {
    // Only show prompt in interactive mode
    let is_interactive = atty::is(atty::Stream::Stdin);

    loop {
        let tokens = get_input_tokenized().unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        #[cfg(debug_assertions)]
        if is_interactive {
            println!("{:?}", tokens);
        }

        let (first, rest) = tokens.split_first().expect("Command not found!");
        let name = first.to_string();
        let raw_args = rest.to_vec();
        let args = Value::from_iter(raw_args.clone());

        // Todo: handle unknown command messages when strings are empty
        if name.is_empty() {
            continue;
        } else if name == "exit" {
            let exit_code = args.get(0, 0);
            process::exit(exit_code);
        } else if name == "echo" {
            println!("{}", args);
        } else if name == "clear" {
            if is_interactive {
                clearscreen::clear().expect("Failed to clear screen");
            }
        } else if name == "type" {
            let exe_name = args.get(0, "");
            if BUILTINS.contains(&exe_name) {
                println!("{} is a shell builtin", exe_name);
            } else {
                match which(exe_name) {
                    Ok(path) => println!("{} is {}", exe_name, path.display()),
                    Err(_) => eprintln!("{}: not found", exe_name),
                }
            }
        } else if name == "pwd" {
            println!(
                "{}",
                env::current_dir()
                    .expect("Failed to get current working directory")
                    .to_string_lossy()
            );
        } else if name == "cd" {
            // Todo: Use https://crates.io/crates/shellexpand
            let home = std::env::var("HOME").expect("Home directory not found");
            let path_string = args.get(0, "~").replace("~", &home);
            let path = Path::new(&path_string);
            env::set_current_dir(path).unwrap_or_else(|_| {
                eprintln!("cd: {}: No such file or directory", path.to_string_lossy())
            });
        } else {
            // Todo: make sure external stdout has a new line at the end
            match execute_external(&name, &raw_args) {
                Ok((stdout, stderr, _)) => {
                    print!("{}", stdout);
                    io::stdout().flush().expect("Failed to flush stdout");
                    eprint!("{}", stderr);
                    io::stderr().flush().expect("Failed to flush stderr");
                }
                Err(e) => {
                    if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                        if io_err.kind() == ErrorKind::NotFound {
                            eprintln!("{}: command not found", name);
                        } else {
                            for cause in e.chain() {
                                eprintln!("{}", cause);
                            }
                        }
                    }
                }
            }
        }
    }
}
