use std::sync::atomic::{AtomicBool, Ordering};

use rustyline::{
    completion::{Completer, Pair},
    error::ReadlineError,
    history::FileHistory,
    Context, Editor, Helper, Highlighter, Hinter, Validator,
};

use crate::{execution::BUILTINS, strings};

#[derive(Helper, Hinter, Validator, Highlighter, Debug)]
pub struct ShellHelper {
    external: Vec<String>,
    builtin: Vec<String>,
    first_tab: AtomicBool,
}
impl ShellHelper {
    pub fn new(path: &[String]) -> Self {
        let external = path.to_vec();
        let builtin = BUILTINS.iter().map(|s| s.to_string()).collect();
        Self {
            external,
            builtin,
            first_tab: AtomicBool::new(true),
        }
    }
}

// Implement Completer
impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
        let prefix = &line[start..pos].to_lowercase();

        let mut matches: Vec<Pair> = self
            .builtin
            .iter()
            .filter(|cmd| cmd.to_lowercase().starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string() + " ",
            })
            .collect();

        // Append external commands that don't duplicate builtins
        matches.extend(
            self.external
                .iter()
                .filter(|cmd| cmd.to_lowercase().starts_with(prefix))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string() + " ",
                }),
        );

        if matches.len() > 1 {
            if self.first_tab.load(Ordering::Relaxed) {
                // First tab press: print bell character
                println!("\x07"); // Bell character
                self.first_tab.store(false, Ordering::Relaxed); // Set flag to false
                return Ok((start, Vec::new())); // Return empty completions to prevent default behavior
            } else {
                // Subsequent tab press: print all matches
                let display = matches
                    .iter()
                    .map(|pair| pair.display.clone())
                    .collect::<Vec<String>>()
                    .join("  ");
                println!("{}", display);
                // Reset first_tab for next completion sequence:
                return Ok((start, Vec::new())); // prevent rustyline from doing any further completion
            }
        }

        Ok((start, matches))
    }
}

pub fn get_input(rl: &mut Editor<ShellHelper, FileHistory>) -> Option<String> {
    let readline = rl.readline("$ ");
    match readline {
        Ok(line) => {
            if let Some(helper) = rl.helper() {
                helper.first_tab.store(true, Ordering::Relaxed);
            }

            Some(line)
        }
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
