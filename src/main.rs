use std::{
    collections::HashMap,
    env::{self, split_paths},
    error::Error,
    fmt::Debug,
    fs::{self, OpenOptions},
    io::{self, PipeReader, PipeWriter, Read, Write, pipe},
    ops::Deref,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::PathBuf,
    process::{self, Child, Command, Stdio},
};

use itertools::Itertools;
use rustyline::{
    CompletionType, Config, Context, Editor, Helper, Highlighter, Hinter, Validator,
    completion::{Completer, Pair},
    config::{BellStyle, Configurer},
    error::ReadlineError,
};

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "hash"];

struct IOPipes {
    input: Box<dyn Read>,
    output: Box<dyn Write>,
    error: Box<dyn Write>,
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

fn handle_echo(args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    pipes
        .output
        .write_all(format!("{}\n", args.join(" ")).as_bytes())
}

fn handle_type(args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    let help_msg = "Usage: type [command: required]\n".as_bytes();

    if args.len() != 1 {
        return pipes.error.write_all(help_msg);
    }

    let Some(cmd) = args.first() else {
        return pipes.error.write_all(help_msg);
    };

    let externals = get_external_executables();

    if BUILTINS.contains(&cmd.as_str()) {
        pipes
            .output
            .write_all(format!("{} is a shell builtin\n", cmd).as_bytes())
    } else if let Some(path) = externals.get(cmd) {
        pipes
            .output
            .write_all(format!("{} is {}\n", cmd, path.to_string_lossy()).as_bytes())
    } else {
        pipes
            .error
            .write_all(format!("{}: not found\n", cmd).as_bytes())
    }
}

fn handle_pwd(args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    if !args.is_empty() {
        return pipes.error.write_all("Usage: pwd\n".as_bytes());
    }

    pipes.output.write_all(
        format!(
            "{}\n",
            env::current_dir()
                .expect("Failed to get current working directory")
                .to_string_lossy()
        )
        .as_bytes(),
    )
}

fn handle_cd(args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    if args.len() > 1 {
        return pipes
            .error
            .write_all("Usage: cd [path: optional (default: ~)]\n".as_bytes());
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
        Ok(()) => Ok(()),
        Err(err) => {
            let msg = err.to_string();
            if msg == "No such file or directory (os error 2)" {
                pipes.error.write_all(
                    format!(
                        "cd: {}: No such file or directory\n",
                        path.to_string_lossy()
                    )
                    .as_bytes(),
                )
            } else {
                pipes.error.write_all(format!("{}\n", msg).as_bytes())
            }
        }
    }
}

fn handle_exit(args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    if args.len() > 1 {
        return pipes
            .error
            .write_all("Usage: exit [exit_code: optional (default: 0)]\n".as_bytes());
    }

    let exit_code = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    process::exit(exit_code);
}
fn handle_external(cmd: &str, args: Vec<String>, pipes: &mut IOPipes) -> io::Result<Option<Child>> {
    let externals = get_external_executables();
    let Some(executable) = externals.get(cmd) else {
        pipes
            .error
            .write_all(format!("{}: command not found\n", cmd).as_bytes());
        return Ok(None);
    };

    // FIXME: Make this work
    let stdin = Stdio::from(pipes.input.deref());
    let stdout = Stdio::from(pipes.output.deref());
    let stderr = Stdio::from(pipes.error.deref());

    let child = match Command::new(executable)
        .arg0(cmd)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
    {
        Ok(output) => output,
        Err(err) => {
            pipes.error.write_all(
                format!("Failed to spawn command '{:?}': {}\n", executable, err).as_bytes(),
            );
            return Ok(None);
        }
    };

    Ok(Some(child))
}

// FIXME: Fix these errors
fn handle_cmd(cmd: &str, args: Vec<String>, pipes: &mut IOPipes) -> io::Result<()> {
    match cmd {
        "echo" => handle_echo(args, pipes),
        "type" => handle_type(args, pipes),
        "pwd" => handle_pwd(args, pipes),
        "cd" => handle_cd(args, pipes),
        "exit" => handle_exit(args, pipes),
        _ => handle_external(cmd, args, pipes),
    }
}

fn checks_redirects(
    redirect_path: Option<String>,
    append_path: Option<String>,
) -> Option<impl Write> {
    if let Some(ref path) = append_path {
        return Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .truncate(false)
                .open(path)
                .unwrap_or_else(|_| panic!("Failed to append to {}!", path)),
        );
    };

    if let Some(ref path) = redirect_path {
        return Some(
            OpenOptions::new()
                .create(true)
                .append(false)
                .truncate(true)
                .open(path)
                .unwrap_or_else(|_| panic!("Failed to write to {}!", path)),
        );
    };

    return None;
}

// FIXME: Wait for children
// OPTIONALLY: spawn threads for builtins with concurrency
fn handle(inputs: Vec<String>) -> io::Result<()> {
    let mut pipes = Vec::new();

    for _ in 0..inputs.len() - 1 {
        pipes.push(Some(pipe()?));
    }

    for (index, input) in inputs.iter().enumerate() {
        let mut parsed = parse_args(input.clone());
        let command = parsed.remove(0);
        let mut args = parsed;
        let redirect_path = get_redirect(&mut args, vec![">".to_string(), "1>".to_string()]);
        let err_redirect_path = get_redirect(&mut args, vec!["2>".to_string()]);
        let append_path = get_redirect(&mut args, vec![">>".to_string(), "1>>".to_string()]);
        let err_append_path = get_redirect(&mut args, vec!["2>>".to_string()]);

        let input_reader: Box<dyn Read>;
        let output_writer: Box<dyn Write>;
        let error_writer: Box<dyn Write>;

        input_reader = if index == 0 {
            Box::new(io::stdin())
        } else {
            Box::new(
                pipes[index - 1]
                    .take()
                    .expect("Pipe reader should be there!")
                    .0,
            )
        };

        output_writer = match checks_redirects(redirect_path, append_path) {
            Some(file) => Box::new(file),
            None => {
                if index + 1 == inputs.len() {
                    Box::new(io::stdout())
                } else {
                    Box::new(pipes[index].take().expect("Pipe writer should be there!").1)
                }
            }
        };

        error_writer = match checks_redirects(err_redirect_path, err_append_path) {
            Some(file) => Box::new(file),
            None => Box::new(io::stderr()),
        };

        let mut io_pipes = IOPipes {
            input: input_reader,
            output: output_writer,
            error: error_writer,
        };

        handle_cmd(command.trim(), args, &mut io_pipes)?;
    }

    Ok(())
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
        handle(inputs)?
    }
}
