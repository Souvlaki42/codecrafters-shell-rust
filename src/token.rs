use std::io::{self, Write};

pub fn get_input_tokenized() -> anyhow::Result<Vec<String>> {
    print!("$ ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim_end_matches(&['\n', '\r'][..]);

    let mut tokens = Vec::new();
    let mut current_token = String::new();

    let mut chars = input.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quote => {
                // Toggle single quote state, do not add quote char
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                // Toggle double quote state, do not add quote char
                in_double_quote = !in_double_quote;
            }
            '\\' => {
                // Handle backslash escapes
                if in_single_quote {
                    // Inside single quotes, backslash is literal
                    current_token.push('\\');
                } else if in_double_quote {
                    // Inside double quotes, backslash escapes only quote or backslash
                    if let Some(&next_char) = chars.peek() {
                        if next_char == '"' || next_char == '\\' {
                            chars.next(); // consume next_char
                            current_token.push(next_char);
                        } else {
                            // Backslash is literal
                            current_token.push('\\');
                        }
                    } else {
                        // Trailing backslash, treat literally
                        current_token.push('\\');
                    }
                } else {
                    // Outside quotes, backslash escapes next char literally
                    if let Some(next_char) = chars.next() {
                        current_token.push(next_char);
                    }
                    // If no next char, trailing backslash ignored or error handled here
                }
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                // Token delimiter outside quotes
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
                // Skip adding the space
            }
            _ => {
                // Normal character or space inside quotes
                current_token.push(c);
            }
        }
    }

    if in_single_quote || in_double_quote {
        anyhow::bail!("Unclosed quote in input");
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    Ok(tokens)
}
