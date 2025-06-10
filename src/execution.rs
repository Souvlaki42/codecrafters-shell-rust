use std::{
    collections::HashMap,
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use crate::value::{Boolean, Integer, Value};

// Todo: add more builtins like:
// - clear (clearscreen crate)
pub const BUILTINS: [&str; 5] = ["echo", "type", "exit", "pwd", "cd"];

pub fn get_external_executables() -> HashMap<String, String> {
    let mut path_executables: HashMap<String, String> = HashMap::new();

    // Add everything we see on the PATH to the other_programs hashmap
    let path = env::var("PATH").expect("Failed to fatch PATH!");
    for dir in path.split(":") {
        // Check if the directory exists
        if !std::path::Path::new(dir).exists() {
            continue;
        }
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let path_str = path.to_str().unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            if path_executables.contains_key(name) || BUILTINS.contains(&name) {
                continue;
            }
            path_executables.insert(name.to_string(), path_str.to_string());
        }
    }
    path_executables
}

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

#[derive(Debug, PartialEq, Eq)]
pub struct ExecuteArgs<'a> {
    pub params: &'a [String],
    pub path: &'a HashMap<String, String>,
    pub input_file: Option<&'a str>,
    pub output_file: Option<&'a str>,
    pub error_file: Option<&'a str>,
    pub append_output: bool,
    pub append_error: bool,
}

impl CommandResult {
    pub fn send_output(
        &self,
        output_path: Option<impl AsRef<Path>>,
        error_path: Option<impl AsRef<Path>>,
        append_output: bool,
        append_error: bool,
    ) {
        match &self.output {
            CommandOutput::Stdout(stdout, flush) => {
                if let Some(path) = output_path {
                    let mut writer = BufWriter::new(Box::new(
                        open_file_create_dirs(path, append_output)
                            .expect("Failed to open output file"),
                    ));
                    if *flush {
                        write!(writer, "{}", stdout).expect("Failed to write stdout (flushed)");
                        writer.flush().unwrap();
                    } else {
                        writeln!(writer, "{}", stdout).expect("Failed to write stdout");
                    }
                } else {
                    // Write to terminal stdout
                    let mut writer = BufWriter::new(Box::new(io::stdout().lock()));
                    if *flush {
                        write!(writer, "{}", stdout).expect("Failed to write stdout (flushed)");
                        writer.flush().unwrap();
                    } else {
                        writeln!(writer, "{}", stdout).expect("Failed to write stdout");
                    }
                }
            }
            CommandOutput::Stderr(stderr, flush) => {
                if let Some(path) = error_path {
                    let mut writer = BufWriter::new(Box::new(
                        open_file_create_dirs(path, append_output)
                            .expect("Failed to open error file"),
                    ));
                    if *flush {
                        write!(writer, "{}", stderr).expect("Failed to write stderr (flushed)");
                        writer.flush().unwrap();
                    } else {
                        writeln!(writer, "{}", stderr).expect("Failed to write stderr");
                    }
                } else {
                    // Write to terminal stderr
                    let mut writer = BufWriter::new(Box::new(io::stderr().lock()));
                    if *flush {
                        write!(writer, "{}", stderr).expect("Failed to write stderr (flushed)");
                        writer.flush().unwrap();
                    } else {
                        writeln!(writer, "{}", stderr).expect("Failed to write stderr");
                    }
                }
            }
            CommandOutput::StdoutAndStderr(stdout, stderr, flush) => {
                // Write stdout
                if let Some(path) = output_path {
                    let mut out = BufWriter::new(Box::new(
                        open_file_create_dirs(path, append_output)
                            .expect("Failed to open output file"),
                    ));
                    if *flush {
                        write!(out, "{}", stdout).expect("Failed to write stdout (flushed)");
                        out.flush().unwrap();
                    } else {
                        writeln!(out, "{}", stdout).expect("Failed to write stdout");
                    }
                } else {
                    let mut out = BufWriter::new(Box::new(io::stdout().lock()));
                    if *flush {
                        write!(out, "{}", stdout).expect("Failed to write stdout (flushed)");
                        out.flush().unwrap();
                    } else {
                        writeln!(out, "{}", stdout).expect("Failed to write stdout");
                    }
                }
                // Write stderr
                if let Some(path) = error_path {
                    let mut err = BufWriter::new(Box::new(
                        open_file_create_dirs(path, append_error)
                            .expect("Failed to open error file"),
                    ));
                    if *flush {
                        write!(err, "{}", stderr).expect("Failed to write stderr (flushed)");
                        err.flush().unwrap();
                    } else {
                        writeln!(err, "{}", stderr).expect("Failed to write stderr");
                    }
                } else {
                    let mut err = BufWriter::new(Box::new(io::stderr().lock()));
                    if *flush {
                        write!(err, "{}", stderr).expect("Failed to write stderr (flushed)");
                        err.flush().unwrap();
                    } else {
                        writeln!(err, "{}", stderr).expect("Failed to write stderr");
                    }
                }
            }
            CommandOutput::NoOutput => {}
        }
    }
}

fn execute_external(
    cmd: &str,
    args: &Vec<String>,
    stdout: Stdio,
    stderr: Stdio,
    stdin: Stdio,
) -> CommandResult {
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

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(format!("Translating stdout error: {}\n", e), true),
                exit_code: 1,
            };
        }
    };
    let stderr = match String::from_utf8(output.stderr) {
        Ok(stderr) => stderr,
        Err(e) => {
            return CommandResult {
                output: CommandOutput::Stderr(format!("Translating stderr error: {}\n", e), true),
                exit_code: 1,
            };
        }
    };
    let status = output.status.code().unwrap_or_default();

    CommandResult {
        output: CommandOutput::StdoutAndStderr(stdout, stderr, true),
        exit_code: status,
    }
}

pub fn execute(args: ExecuteArgs) -> CommandResult {
    let ExecuteArgs {
        params,
        path,
        input_file,
        output_file,
        error_file,
        append_output,
        append_error,
    } = args;

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

    let stdout = match output_file {
        Some(path_str) => {
            let output_path = PathBuf::from(path_str);
            match open_file_create_dirs(output_path, append_output) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    return CommandResult {
                        output: CommandOutput::Stderr(
                            format!("Failed to open output file: {}\n", e),
                            true,
                        ),
                        exit_code: 1,
                    };
                }
            }
        }
        None => Stdio::inherit(),
    };

    let stderr = match error_file {
        Some(path_str) => {
            let error_path = PathBuf::from(path_str);
            match open_file_create_dirs(error_path, append_error) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    return CommandResult {
                        output: CommandOutput::Stderr(
                            format!("Failed to open error file: {}\n", e),
                            true,
                        ),
                        exit_code: 1,
                    };
                }
            }
        }
        None => Stdio::inherit(),
    };

    let (first, rest) = params.split_first().expect("Command not found!");
    let name = first.to_string();
    let args = rest.to_vec();

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
    } else if name == "type" {
        let exe_name = value.get(0, "");
        if BUILTINS.contains(&exe_name) {
            return CommandResult {
                output: CommandOutput::Stdout(format!("{} is a shell builtin", exe_name), false),
                exit_code: 0,
            };
        } else {
            match path.get(exe_name) {
                Some(path) => CommandResult {
                    output: CommandOutput::Stdout(format!("{} is {}", exe_name, path), false),
                    exit_code: 0,
                },
                None => CommandResult {
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
    } else if path.get(&name).is_none() {
        return CommandResult {
            output: CommandOutput::Stderr(format!("{}: command not found\n", name), true),
            exit_code: 127,
        };
    } else {
        execute_external(&name, &args.to_vec(), stdout, stderr, stdin)
    }
}
