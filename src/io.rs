use std::fs::{File, OpenOptions};
use std::io::{self, Read, StderrLock, StdinLock, StdoutLock, Write};
use std::path::Path;

pub fn open_file_create_dirs(path: impl AsRef<Path>, truncate: bool) -> io::Result<File> {
    let path = path.as_ref();

    if let Some(parent_dir) = path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    let mut open_options = OpenOptions::new();

    open_options.read(true).write(true).create(true);

    if truncate {
        open_options.truncate(true);
    }

    open_options.open(path)
}

/// Enum representing a reader: either a file or stdin
#[allow(dead_code)]
pub enum Reader<'a> {
    File(File),
    Stdin(StdinLock<'a>),
}

impl<'a> Read for Reader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Reader::File(file) => file.read(buf),
            Reader::Stdin(stdin_lock) => stdin_lock.read(buf),
        }
    }
}

/// Enum representing a writer: stdout, stderr, or a file
pub enum Writer<'a> {
    Stdout(StdoutLock<'a>),
    Stderr(StderrLock<'a>),
    File(File),
}

impl<'a> Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Writer::Stdout(stdout_lock) => stdout_lock.write(buf),
            Writer::Stderr(stderr_lock) => stderr_lock.write(buf),
            Writer::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Writer::Stdout(stdout_lock) => stdout_lock.flush(),
            Writer::Stderr(stderr_lock) => stderr_lock.flush(),
            Writer::File(file) => file.flush(),
        }
    }
}

/// Factory functions to create readers and writers
pub struct IO;

impl IO {
    /// Create a reader from an optional file path or stdin
    #[allow(dead_code)]
    pub fn create_reader<'a>(path: Option<&str>) -> io::Result<Reader<'a>> {
        if let Some(path) = path {
            let file = open_file_create_dirs(path, false)?; // no truncate, might want to read
            Ok(Reader::File(file))
        } else {
            let stdin = io::stdin();
            Ok(Reader::Stdin(stdin.lock()))
        }
    }

    /// Create a writer to stdout, stderr, or a file
    pub fn create_writer<'a>(path: Option<&str>, error: bool) -> io::Result<Writer<'a>> {
        if let Some(path) = path {
            let truncate = !error; // Don't truncate stderr when file redirecting

            // This is correct now
            let file = open_file_create_dirs(path, truncate)?;
            Ok(Writer::File(file))
        } else if error {
            let stderr = io::stderr();
            Ok(Writer::Stderr(stderr.lock()))
        } else {
            let stdout = io::stdout();
            Ok(Writer::Stdout(stdout.lock()))
        }
    }
}
