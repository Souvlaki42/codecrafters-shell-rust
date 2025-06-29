use std::{
    collections::HashMap,
    env::{self, split_paths},
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::Path,
    process::{self, Command},
};

use super::{
    rw::RW,
    value::{Boolean, Integer, Value},
};

pub const BUILTINS: [&str; 5] = ["echo", "type", "exit", "pwd", "cd"];

pub fn get_external_executables() -> (HashMap<String, String>, Vec<String>) {
    let mut path_executables = HashMap::new();

    let path = env::var("PATH").expect("Failed to fatch PATH!");
    for dir in split_paths(&path) {
        if !dir.exists() {
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

    let path_keys = path_executables.keys().map(|k| k.to_string()).collect();
    (path_executables, path_keys)
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
#[derive(Debug, PartialEq, Eq, Default)]
pub enum CommandOutput {
    Stdout(String, Boolean),
    Stderr(String, Boolean),
    StdoutAndStderr(String, String, Boolean),
    #[default]
    NoOutput,
}

#[derive(Debug)]
pub enum ExecutionOutput {
    Builtin(CommandResult),
    External(process::Child),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub output: CommandOutput,
    pub exit_code: Integer,
}

impl Default for ExecutionOutput {
    fn default() -> Self {
        Self::Builtin(CommandResult {
            output: CommandOutput::NoOutput,
            exit_code: 0,
        })
    }
}

#[derive(Debug)]
pub struct ExecuteArgs<'a> {
    pub params: &'a [String],
    pub path: &'a HashMap<String, String>,
    pub stdin: &'a mut RW,
    pub stdout: &'a mut RW,
    pub stderr: &'a mut RW,
}

impl CommandResult {
    pub fn write_output(&self, out_writer: RW, error_writer: RW) {
        match &self.output {
            CommandOutput::Stdout(output, flush) => {
                let mut writer = BufWriter::from(out_writer);
                if *flush {
                    write!(writer, "{}", output).expect("Failed to write output (flushed)");
                    writer.flush().unwrap();
                } else {
                    writeln!(writer, "{}", output).expect("Failed to write output");
                }
            }
            CommandOutput::Stderr(error, flush) => {
                let mut writer = BufWriter::from(error_writer);
                if *flush {
                    write!(writer, "{}", error).expect("Failed to write error (flushed)");
                    writer.flush().unwrap();
                } else {
                    writeln!(writer, "{}", error).expect("Failed to write error");
                }
            }
            CommandOutput::StdoutAndStderr(output, error, flush) => {
                let mut out = BufWriter::from(out_writer);
                let mut err = BufWriter::from(error_writer);
                if *flush {
                    write!(out, "{}", output).expect("Failed to write output (flushed)");
                    out.flush().unwrap();
                } else {
                    writeln!(out, "{}", output).expect("Failed to write output");
                }

                if *flush {
                    write!(err, "{}", error).expect("Failed to write error (flushed)");
                    err.flush().unwrap();
                } else {
                    writeln!(err, "{}", error).expect("Failed to write error");
                }
            }
            CommandOutput::NoOutput => {}
        }
    }
}

pub fn finalize_executions<T>(execs: T) -> CommandResult
where
    T: IntoIterator<Item = ExecutionOutput>,
{
    let mut iterator = execs.into_iter().peekable();
    if iterator.peek().is_none() {
        return CommandResult {
            output: CommandOutput::NoOutput,
            exit_code: 0,
        };
    }

    while let Some(exec) = iterator.next() {
        if iterator.peek().is_none() {
            return match exec {
                ExecutionOutput::Builtin(output) => output,
                ExecutionOutput::External(child) => {
                    let output = match child.wait_with_output() {
                        Ok(output) => output,
                        Err(e) => {
                            return CommandResult {
                                output: CommandOutput::Stderr(
                                    format!("Retrieving output error: {}\n", e),
                                    true,
                                ),
                                exit_code: 1,
                            };
                        }
                    };

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let status = output.status.code().unwrap_or_default();

                    CommandResult {
                        output: CommandOutput::StdoutAndStderr(stdout, stderr, true),
                        exit_code: status,
                    }
                }
            };
        }

        if let ExecutionOutput::External(mut child) = exec {
            if let Err(e) = child.wait() {
                return CommandResult {
                    output: CommandOutput::Stderr(
                        format!("Error waiting for intermediate command: {}\n", e),
                        true,
                    ),
                    exit_code: 1,
                };
            }
        }
        // If it was a Builtin, we do nothing and move to the next part of the pipe.
    }

    unreachable!("The loop will always return on the last item.");
}

pub fn execute(
    ExecuteArgs {
        params,
        path,
        stdin,
        stdout,
        stderr,
    }: ExecuteArgs,
) -> ExecutionOutput {
    let (first, rest) = params.split_first().expect("Command not found!");
    let name = first.to_string();
    let args = rest.to_vec();

    let value = Value::from_iter(args.to_vec());
    if name.is_empty() {
        ExecutionOutput::default()
    } else if name == "exit" {
        let exit_code = value.get(0, 0);
        process::exit(exit_code);
    } else if name == "echo" {
        return ExecutionOutput::Builtin(CommandResult {
            output: CommandOutput::Stdout(format!("{}", value), false),
            exit_code: 0,
        });
    } else if name == "type" {
        let exe_name = value.get(0, "");
        if BUILTINS.contains(&exe_name) {
            return ExecutionOutput::Builtin(CommandResult {
                output: CommandOutput::Stdout(format!("{} is a shell builtin", exe_name), false),
                exit_code: 0,
            });
        } else {
            match path.get(exe_name) {
                Some(path) => ExecutionOutput::Builtin(CommandResult {
                    output: CommandOutput::Stdout(format!("{} is {}", exe_name, path), false),
                    exit_code: 0,
                }),
                None => ExecutionOutput::Builtin(CommandResult {
                    output: CommandOutput::Stderr(format!("{}: not found", exe_name), false),
                    exit_code: 1,
                }),
            }
        }
    } else if name == "pwd" {
        return ExecutionOutput::Builtin(CommandResult {
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
        });
    } else if name == "cd" {
        let home = env::var("HOME").expect("Home directory not found");
        let path_string = value.get(0, "~").replace("~", &home);
        let path = Path::new(&path_string);
        match env::set_current_dir(path) {
            Ok(_) => ExecutionOutput::Builtin(CommandResult {
                output: CommandOutput::NoOutput,
                exit_code: 0,
            }),
            Err(_) => ExecutionOutput::Builtin(CommandResult {
                output: CommandOutput::Stderr(
                    format!("cd: {}: No such file or directory", path.to_string_lossy()),
                    false,
                ),
                exit_code: 1,
            }),
        }
    } else if path.get(&name).is_none() {
        return ExecutionOutput::Builtin(CommandResult {
            output: CommandOutput::Stderr(format!("{}: command not found\n", name), true),
            exit_code: 127,
        });
    } else {
        let process = Command::new(&name)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .args(args)
            .spawn();

        let child = match process {
            Ok(process) => process,
            Err(e) => {
                return ExecutionOutput::Builtin(CommandResult {
                    output: CommandOutput::Stderr(
                        format!("Failed to spawn command '{}': {}\n", &name, e),
                        true,
                    ),
                    exit_code: 1,
                });
            }
        };

        ExecutionOutput::External(child)
    }
}
