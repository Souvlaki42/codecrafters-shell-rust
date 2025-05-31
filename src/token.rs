use crate::strings;
use std::io::{self, Write};

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim_end_matches(&['\n', '\r'][..]);

    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let chars = input.chars().peekable();
    let mut in_quote: Option<char> = None;

    // First pass: split into tokens while preserving quotes
    for c in chars {
        match c {
            '\'' | '"' if in_quote.is_none() => {
                // Start of quote
                in_quote = Some(c);
                current_token.push(c);
            }
            c if in_quote == Some(c) => {
                // End of quote
                current_token.push(c);
                in_quote = None;
            }
            ' ' | '\t' if in_quote.is_none() => {
                // Token delimiter outside quotes
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
            }
            _ => {
                // Include everything else, including spaces inside quotes
                current_token.push(c);
            }
        }
    }

    if in_quote.is_some() {
        anyhow::bail!("Unclosed quote in input");
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    // Second pass: process each token to handle quotes and escapes
    tokens
        .into_iter()
        .map(|token| strings::process_string(&token))
        .collect()
}
