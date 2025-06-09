use rustyline::{
    completion::{Completer, Pair},
    error::ReadlineError,
    history::FileHistory,
    Context, Editor, Helper, Highlighter, Hinter, Validator,
};

use crate::{execution::BUILTINS, strings};

#[derive(Debug, Helper, Validator, Hinter, Highlighter)]
pub struct Shell {
    commands: Vec<String>,
}

impl Shell {
    pub fn new(commands: Vec<String>) -> Self {
        Self {
            commands: [BUILTINS.iter().map(|s| s.to_string()).collect(), commands].concat(),
        }
    }
}

impl Completer for Shell {
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

pub fn get_input(rl: &mut Editor<Shell, FileHistory>, prompt: &str) -> Option<String> {
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

pub fn tokenize(input: &str) -> anyhow::Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let chars = input.chars().peekable();

    let mut in_quote: Option<char> = None;
    let mut escaped = false;

    for c in chars {
        if escaped {
            // Previous char was backslash, so push both backslash and this char literally
            current_token.push('\\');
            current_token.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
            }
            '\'' | '"' if in_quote.is_none() => {
                in_quote = Some(c);
                current_token.push(c);
            }
            c if in_quote == Some(c) => {
                in_quote = None;
                current_token.push(c);
            }
            ' ' | '\t' if in_quote.is_none() => {
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
            }
            _ => {
                current_token.push(c);
            }
        }
    }

    if escaped {
        anyhow::bail!("Trailing escape character");
    }

    if in_quote.is_some() {
        anyhow::bail!("Unclosed quote in input");
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    // Now process tokens with strings::process_string to handle quoting and unescaping
    tokens
        .into_iter()
        .map(|token| strings::process_string(&token))
        .collect()
}
