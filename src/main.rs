use std::process;
use utils::{get_input_tokenized, Arguments, BUILTINS};

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
        if cmd == "exit" {
            let exit_code = args.get(0, 0);
            process::exit(exit_code);
        } else if cmd == "echo" {
            let values = args.get_all();
            println!("{}", values);
        } else if cmd == "type" {
            let cmd_name = args.get(0, "".to_string());
            if BUILTINS.contains(&cmd_name.as_str()) {
                println!("{} is a shell builtin", cmd_name);
            } else {
                println!("{}: not found", cmd_name);
            }
        } else if !cmd.is_empty() {
            println!("{}: command not found", cmd);
        } else {
            continue;
        }
    }
}
