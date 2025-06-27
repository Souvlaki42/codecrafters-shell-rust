use std::{
    io::{self, BufReader, BufWriter, PipeReader, PipeWriter, Read, Write},
    path::PathBuf,
    process::Stdio,
};

use super::execution::open_file_create_dirs;

#[derive(Debug)]
pub enum RW {
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
impl From<&mut RW> for Stdio {
    fn from(io: &mut RW) -> Self {
        match io {
            RW::File(file_path, append) => {
                let path = PathBuf::from(file_path.clone());
                match open_file_create_dirs(path, *append) {
                    Ok(file) => Stdio::from(file),
                    Err(_) => Stdio::inherit(),
                }
            }
            RW::Pipe => Stdio::piped(),
            RW::Null => Stdio::null(),
            RW::RPipe(ref mut pipe) => Stdio::from(pipe.take().expect("PipeReader already taken")),
            RW::WPipe(ref mut pipe) => Stdio::from(pipe.take().expect("PipeWriter already taken")),
            _ => Stdio::inherit(),
        }
    }
}

/// Implement conversion from IO to BufWriter<Box<dyn Write>>
impl From<RW> for BufWriter<Box<dyn Write>> {
    fn from(io: RW) -> Self {
        match io {
            RW::File(file_path, append) => {
                let path = PathBuf::from(file_path);
                match open_file_create_dirs(path, append) {
                    Ok(file) => BufWriter::new(Box::new(file)),
                    Err(_) => BufWriter::new(Box::new(io::sink())),
                }
            }
            RW::Stdout => BufWriter::new(Box::new(io::stdout())),
            RW::Stderr => BufWriter::new(Box::new(io::stderr())),
            RW::WPipe(mut pipe) => {
                BufWriter::new(Box::new(pipe.take().expect("PipeWriter already taken")))
            }
            _ => BufWriter::new(Box::new(io::sink())),
        }
    }
}

/// Implement conversion from IO to BufReader<Box<dyn Read>>
impl From<RW> for BufReader<Box<dyn Read>> {
    fn from(io: RW) -> Self {
        match io {
            RW::File(file_path, append) => {
                let path = PathBuf::from(file_path);
                match open_file_create_dirs(path, append) {
                    Ok(file) => BufReader::new(Box::new(file)),
                    Err(_) => BufReader::new(Box::new(io::empty())),
                }
            }
            RW::Stdin => BufReader::new(Box::new(io::stdin())),
            RW::RPipe(mut pipe) => {
                BufReader::new(Box::new(pipe.take().expect("PipeReader already taken")))
            }
            _ => BufReader::new(Box::new(io::empty())),
        }
    }
}
