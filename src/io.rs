use std::{
    io::{self, BufReader, BufWriter, PipeReader, PipeWriter, Read, Write},
    path::PathBuf,
    process::Stdio,
};

use crate::execution::open_file_create_dirs;

#[derive(Debug)]
pub enum IO {
    Stdout,
    Stderr,
    Stdin,
    File(String, bool),
    RPipe(Option<PipeReader>),
    WPipe(Option<PipeWriter>),
    Pipe,
    Null,
}

/// Implement conversion from IO to Stdio
impl From<&mut IO> for Stdio {
    fn from(io: &mut IO) -> Self {
        match io {
            IO::File(file_path, append) => {
                let path = PathBuf::from(file_path.clone());
                match open_file_create_dirs(path, *append) {
                    Ok(file) => Stdio::from(file),
                    Err(_) => Stdio::inherit(),
                }
            }
            IO::Pipe => Stdio::piped(),
            IO::Null => Stdio::null(),
            IO::RPipe(ref mut pipe) => Stdio::from(pipe.take().expect("PipeReader already taken")),
            IO::WPipe(ref mut pipe) => Stdio::from(pipe.take().expect("PipeWriter already taken")),
            _ => Stdio::inherit(),
        }
    }
}

/// Implement conversion from IO to BufWriter<Box<dyn Write>>
impl From<IO> for BufWriter<Box<dyn Write>> {
    fn from(io: IO) -> Self {
        match io {
            IO::File(file_path, append) => {
                let path = PathBuf::from(file_path);
                match open_file_create_dirs(path, append) {
                    Ok(file) => BufWriter::new(Box::new(file)),
                    Err(_) => BufWriter::new(Box::new(io::sink())),
                }
            }
            IO::Stdout => BufWriter::new(Box::new(io::stdout())),
            IO::Stderr => BufWriter::new(Box::new(io::stderr())),
            IO::WPipe(mut pipe) => {
                BufWriter::new(Box::new(pipe.take().expect("PipeWriter already taken")))
            }
            _ => BufWriter::new(Box::new(io::sink())),
        }
    }
}

/// Implement conversion from IO to BufReader<Box<dyn Read>>
impl From<IO> for BufReader<Box<dyn Read>> {
    fn from(io: IO) -> Self {
        match io {
            IO::File(file_path, append) => {
                let path = PathBuf::from(file_path);
                match open_file_create_dirs(path, append) {
                    Ok(file) => BufReader::new(Box::new(file)),
                    Err(_) => BufReader::new(Box::new(io::empty())),
                }
            }
            IO::Stdin => BufReader::new(Box::new(io::stdin())),
            IO::RPipe(mut pipe) => {
                BufReader::new(Box::new(pipe.take().expect("PipeReader already taken")))
            }
            _ => BufReader::new(Box::new(io::empty())),
        }
    }
}
