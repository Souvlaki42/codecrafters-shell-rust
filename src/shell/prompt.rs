use rustyline::{
    completion::{Completer, Pair},
    error::ReadlineError,
    history::FileHistory,
    Context, Editor, Helper, Highlighter, Hinter, Validator,
};

use super::execution::BUILTINS;

#[derive(Debug, Helper, Validator, Hinter, Highlighter)]
pub struct Prompt {
    commands: Vec<String>,
}

impl Prompt {
    pub fn new(externals: Vec<String>) -> Self {
        Self {
            commands: [BUILTINS.iter().map(|s| s.to_string()).collect(), externals].concat(),
        }
    }
}

impl Completer for Prompt {
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

pub fn get_input(rl: &mut Editor<Prompt, FileHistory>, prompt: &str) -> Option<String> {
    let readline = rl.readline(prompt);
    match readline {
        Ok(line) => Some(line),
        Err(ReadlineError::Interrupted) => {
            println!("CTRL-C");
            None
        }
        Err(ReadlineError::Eof) => {
            println!("CTRL-D");
            None
        }
        Err(err) => {
            eprintln!("Error: {:?}", err);
            None
        }
    }
}
