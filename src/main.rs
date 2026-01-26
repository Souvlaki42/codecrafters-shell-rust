use std::{
    collections::HashMap,
    env::{self, split_paths},
    fmt::Debug,
    fs::{self, File, OpenOptions},
    io::{self, PipeReader, PipeWriter, Read, Write, pipe},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::PathBuf,
    process::{self, Child, Command, Stdio},
    thread::{self, JoinHandle},
};

use itertools::Itertools;
use rustyline::{
    CompletionType, Config, Context, Editor, Helper, Highlighter, Hinter, Validator,
    completion::{Completer, Pair},
    config::{BellStyle, Configurer},
    error::ReadlineError,
    history::FileHistory,
};

const BUILTINS: [&str; 6] = ["echo", "type", "exit", "pwd", "cd", "history"];

type IOJoinHandle = JoinHandle<io::Result<()>>;

#[derive(Debug)]
enum IOSource {
    PipeReader(PipeReader),
    PipeWriter(PipeWriter),
    File(File),
    Stdout,
    Stdin,
    Stderr,
}

impl From<IOSource> for Stdio {
    fn from(value: IOSource) -> Self {
        match value {
            IOSource::PipeReader(reader) => Self::from(reader),
            IOSource::PipeWriter(writer) => Self::from(writer),
            IOSource::File(file) => Self::from(file),
            IOSource::Stdout => Self::inherit(),
            IOSource::Stdin => Self::inherit(),
            IOSource::Stderr => Self::inherit(),
        }
    }
}

impl Write for IOSource {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            IOSource::PipeReader(_) => unreachable!(),
            IOSource::PipeWriter(writer) => writer.write_all(buf),
            IOSource::File(file) => file.write_all(buf),
            IOSource::Stdout => io::stdout().write_all(buf),
            IOSource::Stdin => unreachable!(),
            IOSource::Stderr => io::stderr().write_all(buf),
        }
    }
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            IOSource::PipeReader(_) => unreachable!(),
            IOSource::PipeWriter(writer) => writer.write(buf),
            IOSource::File(file) => file.write(buf),
            IOSource::Stdout => io::stdout().write(buf),
            IOSource::Stdin => unreachable!(),
            IOSource::Stderr => io::stderr().write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            IOSource::PipeReader(_) => unreachable!(),
            IOSource::PipeWriter(writer) => writer.flush(),
            IOSource::File(file) => file.flush(),
            IOSource::Stdout => io::stdout().flush(),
            IOSource::Stdin => unreachable!(),
            IOSource::Stderr => io::stderr().flush(),
        }
    }
}

impl Read for IOSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            IOSource::PipeReader(reader) => reader.read(buf),
            IOSource::PipeWriter(_) => unreachable!(),
            IOSource::File(file) => file.read(buf),
            IOSource::Stdout => unreachable!(),
            IOSource::Stdin => io::stdin().read(buf),
            IOSource::Stderr => unreachable!(),
        }
    }
}

struct IOPipes {
    #[allow(dead_code)]
    input: IOSource,
    output: IOSource,
    error: IOSource,
}

#[derive(Debug, Helper, Validator, Highlighter, Hinter)]
struct ShellHelper;

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

        let builtins = BUILTINS.map(String::from).to_vec();
        let executables = get_external_executables();

        let mut commands = Vec::from_iter(executables.keys().cloned());
        commands.extend(builtins);

        let mut matches: Vec<Pair> = commands
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

fn handle_history(args: Vec<String>, pipes: &mut IOPipes, history: Vec<String>) -> io::Result<()> {
    let help_msg = "Usage: history [number of entries: optional (default: all)]\n".as_bytes();

    if args.len() > 1 {
        return pipes.error.write_all(help_msg);
    }

    let entries = match args.first() {
        Some(arg) => {
            let Ok(number) = arg.parse() else {
                return pipes.error.write_all(help_msg);
            };
            history.iter().rev().take(number).collect_vec()
        }
        None => history.iter().collect_vec(),
    };

    for (i, entry) in entries.iter().enumerate() {
        pipes
            .output
            .write_all(format!("    {}  {}\n", i + 1, entry).as_bytes())?;
    }
    Ok(())
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
fn handle_external(
    cmd: &str,
    args: Vec<String>,
    input: IOSource,
    output: IOSource,
    mut error: IOSource,
) -> io::Result<Option<Child>> {
    let externals = get_external_executables();
    let Some(executable) = externals.get(cmd) else {
        error.write_all(format!("{}: command not found\n", cmd).as_bytes())?;
        return Ok(None);
    };

    let child = match Command::new(executable)
        .arg0(cmd)
        .args(args)
        .stdin(input)
        .stdout(output)
        .stderr(error)
        .spawn()
    {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Failed to spawn '{:?}': {}", executable, err);
            return Ok(None);
        }
    };

    Ok(Some(child))
}

fn handle_cmd(
    cmd: &str,
    args: Vec<String>,
    editor: &mut Editor<ShellHelper, FileHistory>,
    input: IOSource,
    output: IOSource,
    error: IOSource,
) -> io::Result<(Option<Child>, Option<IOJoinHandle>)> {
    match cmd {
        "echo" => {
            let handle = thread::spawn(move || {
                handle_echo(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                )
            });
            Ok((None, Some(handle)))
        }
        "type" => {
            let handle = thread::spawn(move || {
                handle_type(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                )
            });
            Ok((None, Some(handle)))
        }
        "pwd" => {
            let handle = thread::spawn(move || {
                handle_pwd(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                )
            });
            Ok((None, Some(handle)))
        }
        "cd" => {
            let handle = thread::spawn(move || {
                handle_cd(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                )
            });
            Ok((None, Some(handle)))
        }
        "exit" => {
            let handle = thread::spawn(move || {
                handle_exit(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                )
            });
            Ok((None, Some(handle)))
        }
        "history" => {
            let history = editor.history().into_iter().cloned().collect_vec();
            let handle = thread::spawn(move || {
                handle_history(
                    args,
                    &mut IOPipes {
                        input,
                        output,
                        error,
                    },
                    history,
                )
            });
            Ok((None, Some(handle)))
        }
        _ => handle_external(cmd, args, input, output, error).map(|c| (c, None)),
    }
}

fn checks_redirects(
    redirect_path: Option<String>,
    append_path: Option<String>,
) -> io::Result<Option<File>> {
    if let Some(ref path) = append_path {
        return OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open(path)
            .map(Some);
    };

    if let Some(ref path) = redirect_path {
        return OpenOptions::new()
            .create(true)
            .append(false)
            .write(true)
            .truncate(true)
            .open(path)
            .map(Some);
    };

    Ok(None)
}

fn handle(inputs: Vec<String>, editor: &mut Editor<ShellHelper, FileHistory>) -> io::Result<()> {
    let mut children = Vec::new();
    let mut handles = Vec::new();

    let mut pipe_readers = Vec::new();
    let mut pipe_writers = Vec::new();

    for _ in 0..inputs.len() - 1 {
        let (reader, writer) = pipe()?;
        pipe_readers.push(Some(reader));
        pipe_writers.push(Some(writer));
    }

    for (index, input) in inputs.iter().enumerate() {
        let mut parsed = parse_args(input.clone());
        let command = parsed.remove(0);
        let mut args = parsed;
        let redirect_path = get_redirect(&mut args, vec![">".to_string(), "1>".to_string()]);
        let err_redirect_path = get_redirect(&mut args, vec!["2>".to_string()]);
        let append_path = get_redirect(&mut args, vec![">>".to_string(), "1>>".to_string()]);
        let err_append_path = get_redirect(&mut args, vec!["2>>".to_string()]);

        let input_reader = if index == 0 {
            IOSource::Stdin
        } else {
            IOSource::PipeReader(
                pipe_readers[index - 1]
                    .take()
                    .expect("Pipe reader should be there!"),
            )
        };

        let output_writer = match checks_redirects(redirect_path, append_path)? {
            Some(file) => IOSource::File(file),
            None => {
                if index + 1 == inputs.len() {
                    IOSource::Stdout
                } else {
                    IOSource::PipeWriter(
                        pipe_writers[index]
                            .take()
                            .expect("Pipe writer should be there!"),
                    )
                }
            }
        };

        let error_writer = match checks_redirects(err_redirect_path, err_append_path)? {
            Some(file) => IOSource::File(file),
            None => IOSource::Stderr,
        };

        handle_cmd(
            command.trim(),
            args,
            editor,
            input_reader,
            output_writer,
            error_writer,
        )
        .map(|(child, handle)| {
            if let Some(c) = child {
                children.push(c);
            }

            if let Some(h) = handle {
                handles.push(h);
            }
        })?;
    }

    for handle in handles {
        handle.join().expect("Failed joining handle")?;
    }

    for mut child in children {
        child.wait()?;
    }

    Ok(())
}

// TODO: status codes
// TODO: input redirection
// TODO: variable expansion
fn main() -> io::Result<()> {
    let shell_helper = ShellHelper {};
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
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        };

        let inputs = line.split("|").map(|s| s.trim().to_string()).collect_vec();
        handle(inputs, &mut editor)?
    }

    Ok(())
}
