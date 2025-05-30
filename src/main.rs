use std::{io::ErrorKind, process};
use utils::{execute_external, get_input_tokenized, Arguments, BUILTINS};
use which::which;

mod utils;
mod value;

fn main() {
    loop {
        let tokens = get_input_tokenized().unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        let args = Arguments::new(tokens);
        let cmd = args.cmd();

        // todo handle unknown command messages when strings are empty
        if cmd.is_empty() {
            continue;
        } else if cmd == "exit" {
            let exit_code = args.get(0, 0);
            process::exit(exit_code);
        } else if cmd == "echo" {
            let values = args.get_all();
            println!("{}", values);
        } else if cmd == "type" {
            let exe_name = args.get(0, "".to_string());
            if BUILTINS.contains(&exe_name.as_str()) {
                println!("{} is a shell builtin", exe_name);
            } else {
                match which(&exe_name) {
                    Ok(path) => println!("{} is {}", exe_name, path.display()),
                    Err(_) => eprintln!("{}: not found", exe_name),
                }
            }
        } else {
            let raw_args = args.get_raw();
            match execute_external(&cmd, raw_args) {
                Ok((stdout, stderr, _)) => {
                    println!("{}", stdout);
                    eprintln!("{}", stderr);
                }
                Err(e) => {
                    if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                        if io_err.kind() == ErrorKind::NotFound {
                            eprintln!("{}: command not found", cmd);
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
