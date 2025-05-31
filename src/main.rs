use std::{
    io::{self, Write},
    process,
};
use utils::{execute_external, get_input_tokenized, Arguments, BUILTINS};
use which::which;

mod utils;
mod value;

fn main() {
    print!("$ ");
    io::stdout().flush().expect("Failed to flush stdout");

    loop {
        let tokens = get_input_tokenized().unwrap_or_else(|e| {
            eprintln!("Tokenizer failed: {}", e);
            process::exit(1);
        });

        let args = Arguments::new(tokens);
        let cmd = args.cmd();

        // Todo handle unknown command messages when strings are empty
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
            if let Ok((stdout, _, _)) = execute_external(&cmd, raw_args) {
                println!("{}", stdout);
            }
        }

        print!("$ ");
        io::stdout().flush().expect("Failed to flush stdout");
    }
}
