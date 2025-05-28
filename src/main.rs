use std::{
    io::{self, Write},
    process,
};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let mut inside_string = false;
        let cmd: Vec<&str> = input
            .split(|c| {
                if c == '\'' || c == '"' {
                    inside_string = !inside_string;
                }
                !inside_string && c == ' '
            })
            .map(|token| token.trim())
            .collect();

        if cmd.len() == 2 && cmd[0] == "exit" {
            process::exit(cmd[1].parse().unwrap())
        } else {
            println!("{}: command not found", input.trim());
        }
    }
}
