use std::io::{self, Write};

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for c in input.trim().chars() {
        if escaped {
            current_token.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
            }
            '\'' | '"' => {
                if quote.is_none() {
                    quote = Some(c)
                } else if quote == Some(c) {
                    quote = None;
                } else {
                    current_token.push(c);
                }
            }
            ' ' => {
                if quote.is_none() {
                    // Only split on space if we're not inside any quotes
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                } else {
                    // Inside quotes, preserve the space
                    current_token.push(c);
                }
            }
            _ => {
                current_token.push(c);
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    Ok(tokens)
}
