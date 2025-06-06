use std::{
    env,
    io::Write,
    path::Path,
    process::{self, Command, Stdio},
};

use which::which;

use crate::{
    io::Writer,
    value::{Boolean, Integer, Value},
};

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "clear"];

/// Define a custom enum for the function's outcome and if it should be flushed.
#[derive(Debug, PartialEq, Eq, Clone)]
enum CommandOutput {
    /// Command completed successfully, only stdout was produced.
    Stdout(String),
    /// Command completed successfully, only stderr was produced.
    Stderr(String),
    /// Command completed successfully, both stdout and stderr were produced.
    /// The first string is stdout, the second is stderr.
    StdoutAndStderr(String, String),
    /// Command completed successfully, but produced no output.
    NoOutput,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandResult {
    output: CommandOutput,
    exit_code: Integer,
    external: Boolean,
}

impl CommandResult {
    // Check `if self.external` and skip `if name = BUILTINS`
    pub fn to_output(&self, out: &mut Writer) {
        // Check external commands and return
        if !self.external {
            return;
        }

        let output = self.output.clone();
        match output {
            CommandOutput::Stdout(stdout) => {
                out.write_all(stdout.as_bytes()).unwrap();
                out.write_all(b"\n").unwrap(); // explicit new line.
                out.flush().unwrap(); // ensure that it is displayed
            }
            CommandOutput::StdoutAndStderr(stdout, _) => {
                out.write_all(stdout.as_bytes()).unwrap();
                out.write_all(b"\n").unwrap(); // explicit new line.
                out.flush().unwrap(); // ensure that it is displayed
            }
            _ => {}
        }
    }

    // Check `if self.external` and skip `if name = BUILTINS`
    pub fn to_error(&self, err: &mut Writer) {
        // Check external commands and return
        if !self.external {
            return;
        }

        let output = self.output.clone();
        match output {
            CommandOutput::Stderr(stderr) => {
                err.write_all(stderr.as_bytes()).unwrap();
                err.write_all(b"\n").unwrap(); // explicit new line.
                err.flush().unwrap(); // ensure that it is displayed
            }
            CommandOutput::StdoutAndStderr(_, stderr) => {
                err.write_all(stderr.as_bytes()).unwrap();
                err.write_all(b"\n").unwrap(); // explicit new line.
                err.flush().unwrap(); // ensure that it is displayed
            }
            _ => {}
        }
    }
}

// Ensure writers have `dyn Write` not the explicit types.
fn execute_external(cmd: &str, args: &Vec<String>, writers: (&Writer, &Writer)) -> CommandResult {
    // Stdio config
    let stdout = match writers.0 {
        Writer::File(file) => Stdio::from(file.try_clone().unwrap()),
        _ => Stdio::piped(), // Use a pipe for stdout
    };
    let stderr = match writers.1 {
        Writer::File(file) => Stdio::from(file.try_clone().unwrap()),
        _ => Stdio::piped(), // Use a pipe for stderr
    };

    // Set cmd
    let mut command = Command::new(cmd);
    command.stdout(stdout); // Set new stdout
    command.stderr(stderr); // Set new stderr
    command.args(args);

    let process = match command.spawn() {
        Ok(process) => process,
        Err(e) => {
            // Check to display the problem with the messages
            let message = format!("Failed to spawn command '{}': {}\n", cmd, e);
            eprintln!("Process problem: {message}");

            return CommandResult {
                output: CommandOutput::Stderr(message),
                exit_code: 1,
                external: true,
            };
        }
    };

    let output = match process.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            let message = format!("Retrieving output error: {}\n", e);
            eprintln!("Wait and get message problem: {message}");

            return CommandResult {
                output: CommandOutput::Stderr(message),
                exit_code: 1,
                external: true,
            };
        }
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(e) => {
            let message = format!("Translating stdout error: {}\n", e);
            eprintln!("Stdout transalation problem: {message}");

            return CommandResult {
                output: CommandOutput::Stderr(message),
                exit_code: 1,
                external: true,
            };
        }
    };
    let stderr = match String::from_utf8(output.stderr) {
        Ok(stderr) => stderr,
        Err(e) => {
            // To test if this is the actual error we receive, println to stderr
            let message = format!("Translating stderr error: {}\n", e);
            eprintln!("Stderr translation problem: {message}");

            return CommandResult {
                output: CommandOutput::Stderr(message),
                exit_code: 1,
                external: true,
            };
        }
    };

    let status = output.status.code().unwrap_or_default();

    CommandResult {
        output: CommandOutput::StdoutAndStderr(stdout, stderr),
        exit_code: status,
        external: true,
    }
}

// The logic should still be the same from the other ones.
pub fn execute(
    name: String,
    args: Value,
    raw_args: Vec<String>,
    writers: (&Writer, &Writer),
) -> CommandResult {
    if name.is_empty() {
        CommandResult {
            output: CommandOutput::NoOutput,
            exit_code: 0,
            external: false,
        }
    } else if name == "exit" {
        let exit_code = args.get(0, 0);
        process::exit(exit_code);
    } else if name == "echo" {
        return CommandResult {
            output: CommandOutput::Stdout(format!("{}", args)),
            exit_code: 0,
            external: false,
        };
    } else if name == "clear" {
        match clearscreen::clear() {
            Ok(_) => {
                return CommandResult {
                    output: CommandOutput::NoOutput,
                    exit_code: 0,
                    external: false,
                }
            }
            Err(e) => {
                return CommandResult {
                    output: CommandOutput::Stderr(format!("Clearing screen error: {}", e)),
                    exit_code: 1,
                    external: false,
                };
            }
        }
    } else if name == "type" {
        let exe_name = args.get(0, "");
        if BUILTINS.contains(&exe_name) {
            return CommandResult {
                output: CommandOutput::Stdout(format!("{} is a shell builtin", exe_name)),
                exit_code: 0,
                external: false,
            };
        } else {
            match which(exe_name) {
                Ok(path) => CommandResult {
                    output: CommandOutput::Stdout(format!("{} is {}", exe_name, path.display())),
                    exit_code: 0,
                    external: false,
                },
                Err(_) => CommandResult {
                    output: CommandOutput::Stderr(format!("{}: not found", exe_name)),
                    exit_code: 1,
                    external: false,
                },
            }
        }
    } else if name == "pwd" {
        return CommandResult {
            output: CommandOutput::Stdout(format!(
                "{}",
                env::current_dir()
                    .expect("Failed to get current working directory")
                    .to_string_lossy()
            )),
            exit_code: 0,
            external: false,
        };
    } else if name == "cd" {
        // Todo: Use https://crates.io/crates/shellexpand
        let home = std::env::var("HOME").expect("Home directory not found");
        let path_string = args.get(0, "~").replace("~", &home);
        let path = Path::new(&path_string);
        match env::set_current_dir(path) {
            Ok(_) => CommandResult {
                output: CommandOutput::NoOutput,
                exit_code: 0,
                external: false,
            },
            Err(_) => CommandResult {
                output: CommandOutput::Stderr(format!(
                    "cd: {}: No such file or directory\n",
                    path.to_string_lossy()
                )),
                exit_code: 1,
                external: false,
            },
        }
    } else {
        // Todo: make sure external stdout has a new line at the end
        execute_external(&name, &raw_args, writers)
    }
}
