use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use which::which;

use crate::value::{Boolean, Integer, Value};

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "clear"];

pub fn open_file_create_dirs(path: impl AsRef<Path>, append: bool) -> io::Result<File> {
    let path = path.as_ref();

    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let mut open_options = OpenOptions::new();
    open_options
        .read(true)
        .write(true)
        .create(true)
        .append(append);

    open_options.open(path)
}

/// Define a custom enum for the function's outcome and if it should be flushed.
#[derive(Debug, PartialEq, Eq)]
pub enum CommandOutput {
    /// Command completed successfully, only stdout was produced.
    Stdout(String, Boolean),
    /// Command completed successfully, only stderr was produced.
    Stderr(String, Boolean),
    /// Command completed successfully, both stdout and stderr were produced.
    /// The first string is stdout, the second is stderr.
    StdoutAndStderr(String, String, Boolean),
    /// Command completed successfully, but produced no output.
    NoOutput,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub output: CommandOutput,
    pub exit_code: Integer,
}

pub fn print_command_output(result: CommandResult, skip_stdout: bool, skip_stderr: bool) {
    let CommandResult { output, .. } = result;
    match output {
        CommandOutput::Stdout(stdout, flush) => {
            if skip_stdout {
                return;
            }
            if flush {
                print!("{}", stdout);
                io::stdout().flush().unwrap();
            } else {
                println!("{}", stdout);
            }
        }
        CommandOutput::Stderr(stderr, flush) => {
            if skip_stderr {
                return;
            }
            if flush {
                eprint!("{}", stderr);
                io::stderr().flush().unwrap();
            } else {
                eprintln!("{}", stderr);
            }
        }
        CommandOutput::StdoutAndStderr(stdout, stderr, flush) => {
            if flush {
                if !skip_stdout {
                    print!("{}", stdout);
                    io::stdout().flush().unwrap();
                }

                if !skip_stderr {
                    eprint!("{}", stderr);
                    io::stderr().flush().unwrap();
                }
            } else {
                if !skip_stdout {
                    println!("{}", stdout);
                }
                if !skip_stderr {
                    eprintln!("{}", stderr);
                }
            }
        }
        CommandOutput::NoOutput => {}
    }
}

fn execute_external(
    cmd: &str,
    args: &Vec<String>,
    input_file: Option<&str>,
    output_file: Option<&str>,
    error_file: Option<&str>,
    append_output: bool,
    append_error: bool,
) -> CommandResult {
    // ===================== INPUT REDIRECTION =========================
    let stdin = match input_file {
        Some(path_str) => {
            let input_path = PathBuf::from(path_str);
            match File::open(input_path) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    return CommandResult {
                        output: CommandOutput::Stderr(
                            format!("Failed to open input file: {}\n", e),
                            true,
                        ),
                        exit_code: 1,
                    };
                }
            }
        }
        None => Stdio::inherit(),
    };

    // ===================== OUTPUT REDIRECTION =========================
    let stdout = Stdio::piped(); // Always pipe stdout
    let stderr = Stdio::piped(); // Always pipe stderr

    // ===================== EXECUTE COMMAND =========================
    let process = Command::new(cmd)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .args(args)
        .spawn();

    let child = match process {
        Ok(process) => process,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(
                    format!("Failed to spawn command '{}': {}\n", cmd, e),
                    true,
                ),
                exit_code: 1,
            };
        }
    };

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(format!("Retrieving output error: {}\n", e), true),
                exit_code: 1,
            };
        }
    };

    let stdout_data = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(format!("Translating stdout error: {}\n", e), true),
                exit_code: 1,
            };
        }
    };
    let stderr_data = match String::from_utf8(output.stderr) {
        Ok(stderr) => stderr,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(format!("Translating stderr error: {}\n", e), true),
                exit_code: 1,
            };
        }
    };

    // Handle redirection to file after successful execution
    if let Some(path_str) = output_file {
        let output_path = PathBuf::from(path_str);
        match open_file_create_dirs(output_path, append_output) {
            Ok(mut file) => {
                if let Err(e) = write!(file, "{}", stdout_data) {
                    return CommandResult {
                        output: CommandOutput::Stderr(
                            format!("Failed to write to output file: {}\n", e),
                            true,
                        ),
                        exit_code: 1,
                    };
                }
            }
            Err(e) => {
                return CommandResult {
                    output: CommandOutput::Stderr(
                        format!("Failed to open output file for writing: {}\n", e),
                        true,
                    ),
                    exit_code: 1,
                };
            }
        }
    }

    if let Some(path_str) = error_file {
        let error_path = PathBuf::from(path_str);
        match open_file_create_dirs(error_path, append_error) {
            Ok(mut file) => {
                if let Err(e) = write!(file, "{}", stderr_data) {
                    return CommandResult {
                        output: CommandOutput::Stderr(
                            format!("Failed to write to error file: {}\n", e),
                            true,
                        ),
                        exit_code: 1,
                    };
                }
            }
            Err(e) => {
                return CommandResult {
                    output: CommandOutput::Stderr(
                        format!("Failed to open error file for writing: {}\n", e),
                        true,
                    ),
                    exit_code: 1,
                };
            }
        }
    }

    let status = output.status.code().unwrap_or_default();

    CommandResult {
        output: CommandOutput::StdoutAndStderr(stdout_data, stderr_data, true),
        exit_code: status,
    }
}

pub fn execute(
    name: String,
    args: &[String],
    input_file: Option<&str>,
    output_file: Option<&str>,
    error_file: Option<&str>,
    append_output: bool,
    append_error: bool,
) -> CommandResult {
    let value = Value::from_iter(args.to_vec());
    if name.is_empty() {
        CommandResult {
            output: CommandOutput::NoOutput,
            exit_code: 0,
        }
    } else if name == "exit" {
        let exit_code = value.get(0, 0);
        process::exit(exit_code);
    } else if name == "echo" {
        return CommandResult {
            output: CommandOutput::Stdout(format!("{}", value), false),
            exit_code: 0,
        };
    } else if name == "clear" {
        match clearscreen::clear() {
            Ok(_) => {
                return CommandResult {
                    output: CommandOutput::NoOutput,
                    exit_code: 0,
                }
            }
            Err(e) => {
                return CommandResult {
                    output: CommandOutput::Stderr(format!("Clearing screen error: {}", e), false),
                    exit_code: 1,
                };
            }
        }
    } else if name == "type" {
        let exe_name = value.get(0, "");
        if BUILTINS.contains(&exe_name) {
            return CommandResult {
                output: CommandOutput::Stdout(format!("{} is a shell builtin", exe_name), false),
                exit_code: 0,
            };
        } else {
            match which(exe_name) {
                Ok(path) => CommandResult {
                    output: CommandOutput::Stdout(
                        format!("{} is {}", exe_name, path.display()),
                        false,
                    ),
                    exit_code: 0,
                },
                Err(_) => CommandResult {
                    output: CommandOutput::Stderr(format!("{}: not found", exe_name), false),
                    exit_code: 1,
                },
            }
        }
    } else if name == "pwd" {
        return CommandResult {
            output: CommandOutput::Stdout(
                format!(
                    "{}",
                    env::current_dir()
                        .expect("Failed to get current working directory")
                        .to_string_lossy()
                ),
                false,
            ),
            exit_code: 0,
        };
    } else if name == "cd" {
        // Todo: Use https://crates.io/crates/shellexpand
        let home = std::env::var("HOME").expect("Home directory not found");
        let path_string = value.get(0, "~").replace("~", &home);
        let path = Path::new(&path_string);
        match env::set_current_dir(path) {
            Ok(_) => CommandResult {
                output: CommandOutput::NoOutput,
                exit_code: 0,
            },
            Err(_) => CommandResult {
                output: CommandOutput::Stderr(
                    format!("cd: {}: No such file or directory", path.to_string_lossy()),
                    false,
                ),
                exit_code: 1,
            },
        }
    } else {
        execute_external(
            &name,
            &args.to_vec(),
            input_file,
            output_file,
            error_file,
            append_output,
            append_error,
        )
    }
}
