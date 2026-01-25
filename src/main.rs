use std::{
    collections::HashMap,
    env::{self, split_paths},
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::PathBuf,
    process::{self, Command, Stdio},
};

use itertools::Itertools;
use rustyline::{
    CompletionType, Config, Context, Editor, Helper, Highlighter, Hinter, Validator,
    completion::{Completer, Pair},
    config::{BellStyle, Configurer},
    error::ReadlineError,
};

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "hash"];

#[derive(Debug, Clone)]
struct Response {
    output: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Helper, Validator, Highlighter, Hinter)]
struct ShellHelper {
    commands: Vec<String>,
}

impl ShellHelper {
    pub fn update_commands(&mut self) {
        let builtins = BUILTINS.map(String::from).to_vec();
        let executables = get_external_executables();

        self.commands = Vec::from_iter(executables.keys().cloned());
        self.commands.extend(builtins);
    }
    pub fn new() -> Self {
        let mut instance = Self {
            commands: Vec::new(),
        };
        instance.update_commands();
        instance
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
        let prefix = &line[start..pos].to_lowercase();

        let mut matches: Vec<Pair> = self
            .commands
            .iter()
            .filter(|cmd| cmd.to_lowercase().starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string() + " ",
            })
            .collect();

        matches.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((start, matches))
    }
}

fn get_external_executables() -> HashMap<String, PathBuf> {
    let path = env::var("PATH").expect("Failed to fetch PATH!");
    let mut results = HashMap::new();
    for dir in split_paths(&path) {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let is_executable = path.is_file()
                && (path
                    .metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false));

            if !is_executable {
                continue;
            }

            if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy().to_string()) {
                results.entry(file_name).or_insert(path);
            }
        }
    }
    results
}

fn parse_args(input: String) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    let mut chars = input.trim().chars().peekable();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && (!in_single) {
            if in_double {
                if let Some(nc) = chars.peek().copied()
                    && (nc == '\"' || nc == '\\')
                {
                    escaped = true;
                    continue;
                }
            } else {
                escaped = true;
                continue;
            }
        }

        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

fn get_redirect(args: &mut Vec<String>, redirect_pipes: Vec<String>) -> Option<String> {
    let pos = args.clone().iter().position(|x| redirect_pipes.contains(x));

    if let Some(pos) = pos {
        let path = Some(args[pos + 1].clone());
        args.remove(pos + 1);
        args.remove(pos);
        path
    } else {
        None
    }
}

fn handle_echo(args: Vec<String>) -> Response {
    Response {
        output: Some(args.join(" ") + "\n"),
        error: None,
    }
}

fn handle_type(args: Vec<String>) -> Response {
    let help_msg = Response {
        output: None,
        error: Some(String::from("Usage: type [command: required]\n")),
    };

    if args.len() != 1 {
        return help_msg;
    }

    let Some(cmd) = args.first() else {
        return help_msg;
    };

    let externals = get_external_executables();

    if BUILTINS.contains(&cmd.as_str()) {
        Response {
            output: Some(format!("{} is a shell builtin\n", cmd)),
            error: None,
        }
    } else if let Some(path) = externals.get(cmd) {
        Response {
            output: Some(format!("{} is {}\n", cmd, path.to_string_lossy())),
            error: None,
        }
    } else {
        Response {
            output: None,
            error: Some(format!("{}: not found\n", cmd)),
        }
    }
}

fn handle_pwd(args: Vec<String>) -> Response {
    if !args.is_empty() {
        return Response {
            output: None,
            error: Some(String::from("Usage: pwd\n")),
        };
    }

    Response {
        output: Some(format!(
            "{}\n",
            env::current_dir()
                .expect("Failed to get current working directory")
                .to_string_lossy()
        )),
        error: None,
    }
}

fn handle_cd(args: Vec<String>) -> Response {
    if args.len() > 1 {
        return Response {
            output: None,
            error: Some(String::from("Usage: cd [path: optional (default: ~)]\n")),
        };
    }

    let default_path = env::home_dir().expect("Couldn't find $HOME path!");
    let path = args
        .first()
        .map(|s| {
            if s == "~" {
                default_path.clone()
            } else {
                PathBuf::from(s)
            }
        })
        .unwrap_or(default_path);

    match env::set_current_dir(&path) {
        Ok(()) => Response {
            output: None,
            error: None,
        },
        Err(err) => {
            let msg = err.to_string();
            if msg == "No such file or directory (os error 2)" {
                Response {
                    output: None,
                    error: Some(format!(
                        "cd: {}: No such file or directory\n",
                        path.to_string_lossy()
                    )),
                }
            } else {
                Response {
                    output: None,
                    error: Some(msg + "\n"),
                }
            }
        }
    }
}

fn handle_exit(args: Vec<String>) -> Response {
    if args.len() > 1 {
        return Response {
            output: None,
            error: Some(String::from(
                "Usage: exit [exit_code: optional (default: 0)]\n",
            )),
        };
    }

    let exit_code = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    process::exit(exit_code);
}
fn handle_external(cmd: &str, args: Vec<String>) -> Response {
    let externals = get_external_executables();
    let Some(executable) = externals.get(cmd) else {
        return Response {
            output: None,
            error: Some(format!("{}: command not found\n", cmd)),
        };
    };

    let result = match Command::new(executable).arg0(cmd).args(args).output() {
        Ok(output) => output,
        Err(err) => {
            return Response {
                output: None,
                error: Some(format!(
                    "Failed to spawn command '{:?}': {}\n",
                    executable, err
                )),
            };
        }
    };

    let output = if result.stdout.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&result.stdout).to_string())
    };
    let error = if result.stderr.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&result.stderr).to_string())
    };

    Response { output, error }
}

fn handle_cmd(cmd: &str, args: Vec<String>) -> Response {
    match cmd {
        "echo" => handle_echo(args),
        "type" => handle_type(args),
        "pwd" => handle_pwd(args),
        "cd" => handle_cd(args),
        "exit" => handle_exit(args),
        _ => handle_external(cmd, args),
    }
}

fn write_output<T: Write>(
    output: Option<String>,
    redirect_path: Option<String>,
    append_path: Option<String>,
    mut default_writer: T,
) -> io::Result<()> {
    if let Some(ref path) = append_path {
        return OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open(path)
            .unwrap_or_else(|_| panic!("Failed to append to {}!", path))
            .write_all(output.clone().unwrap_or_default().as_bytes());
    };

    if let Some(ref path) = redirect_path {
        return OpenOptions::new()
            .create(true)
            .append(false)
            .truncate(true)
            .open(path)
            .unwrap_or_else(|_| panic!("Failed to write to {}!", path))
            .write_all(output.clone().unwrap_or_default().as_bytes());
    };

    default_writer.write_all(output.unwrap_or_default().as_bytes())
}

fn handle(input: String) -> io::Result<()> {
    let mut parsed = parse_args(input);
    let command = parsed.remove(0);
    let mut args = parsed;
    let redirect_path = get_redirect(&mut args, vec![">".to_string(), "1>".to_string()]);
    let err_redirect_path = get_redirect(&mut args, vec!["2>".to_string()]);
    let append_path = get_redirect(&mut args, vec![">>".to_string(), "1>>".to_string()]);
    let err_append_path = get_redirect(&mut args, vec!["2>>".to_string()]);
    let result = handle_cmd(command.trim(), args);

    write_output(result.output, redirect_path, append_path, io::stdout())?;
    write_output(
        result.error,
        err_redirect_path,
        err_append_path,
        io::stderr(),
    )?;

    Ok(())
}

fn handle_pipelines(commands: Vec<String>) {
    let mut previous = None;
    let mut children = Vec::new();
    for cmd_str in &commands {
        let mut parts = cmd_str.split_whitespace();
        let program = parts.next().expect("Program not found!");
        let args = parts.collect_vec();

        let mut cmd = Command::new(program);
        cmd.args(args);

        if let Some(stdout) = previous {
            cmd.stdin(stdout);
        }

        if cmd_str != commands.last().expect("Commands not found!") {
            cmd.stdout(Stdio::piped());
        }

        let mut child = cmd.spawn().expect("Failed to spawn child!");
        previous = child.stdout.take().map(Stdio::from);
        children.push(child);
    }

    for mut child in children {
        child.wait().expect("Child failed!");
    }
}

// TODO: status codes, flushing
fn main() -> io::Result<()> {
    let shell_helper = ShellHelper::new();
    let config = Config::builder()
        .bell_style(BellStyle::Audible)
        .completion_type(CompletionType::List)
        .build();
    let mut editor = Editor::with_config(config).expect("Failed to setup the prompt");
    editor.set_helper(Some(shell_helper));
    editor.set_history_ignore_space(true);
    editor.set_auto_add_history(true);

    loop {
        let line = match editor.readline("$ ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                continue;
            }
            Err(err) => {
                println!("Error: {err:?}");
                continue;
            }
        };

        let inputs = line.split("|").map(|s| s.trim().to_string()).collect_vec();
        if inputs.len() == 1 {
            handle(line)?;
        } else {
            handle_pipelines(inputs);
        }
    }
}
