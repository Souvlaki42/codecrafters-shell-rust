use std::io::{self, Write};

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut quote_stack = Vec::new(); // Stack to track nested quotes
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
                match quote_stack.last() {
                    None => {
                        // Not in any quotes, start a new quoted string
                        quote_stack.push(c);
                    }
                    Some(&last_quote) => {
                        if c == last_quote {
                            // Found matching closing quote
                            quote_stack.pop();
                        } else {
                            // Different quote type, treat as literal
                            current_token.push(c);
                        }
                    }
                }
            }
            ' ' => {
                if quote_stack.is_empty() {
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
