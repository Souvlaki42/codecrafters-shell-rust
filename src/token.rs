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
    let mut escaped = false;

    for c in chars {
        if escaped {
            // Previous char was backslash, so push current char literally
            current_token.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' => {
                // Escape next char
                escaped = true;
            }
            '\'' | '"' if in_quote.is_none() => {
                // Start quote
                in_quote = Some(c);
                current_token.push(c);
            }
            c if in_quote == Some(c) => {
                // End quote
                in_quote = None;
                current_token.push(c);
            }
            ' ' | '\t' if in_quote.is_none() => {
                // Space outside quotes and not escaped: token delimiter
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

    // Process tokens to handle quotes and escapes
    tokens
        .into_iter()
        .map(|token| strings::process_string(&token))
        .collect()
}
