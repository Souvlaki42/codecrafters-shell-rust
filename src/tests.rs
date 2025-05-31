use anyhow::Result;
use std::process::{Command, Output};

/// Runs a command in our shell and returns the output
fn run_shell_command(cmd: &str) -> Result<Output> {
    let output = Command::new("cargo").args(["run", "--", cmd]).output()?;
    Ok(output)
}

/// Helper to run a command and get stdout as string, removing the prompt
fn run_and_get_stdout(cmd: &str) -> Result<String> {
    let output = run_shell_command(cmd)?;
    let stdout = String::from_utf8(output.stdout)?;

    // Debug: Print raw output
    #[cfg(test)]
    println!("Raw output for '{}':\n{}", cmd, stdout);

    // Split into lines and process
    let lines: Vec<&str> = stdout.lines().collect();

    // Find the last line that starts with $ (the prompt)
    let prompt_index = lines
        .iter()
        .rposition(|line| line.starts_with('$'))
        .unwrap_or(0);

    // Take all lines after the last prompt
    let output_lines: Vec<&str> = lines
        .into_iter()
        .skip(prompt_index + 1)
        .filter(|line| !line.starts_with('[')) // Filter debug output
        .collect();

    // Join lines and ensure proper newline
    let output = output_lines.join("\n");
    Ok(if output.is_empty() {
        output
    } else {
        output + "\n"
    })
}

#[test]
fn test_quote_handling() -> Result<()> {
    // Test single quotes
    assert_eq!(
        run_and_get_stdout("echo 'hello\\'world'")?,
        "hello\\'world\n"
    );
    assert_eq!(run_and_get_stdout("echo 'hello\"world'")?, "hello\"world\n");
    assert_eq!(run_and_get_stdout("echo 'hello\\world'")?, "hello\\world\n");

    // Test double quotes
    assert_eq!(
        run_and_get_stdout("echo \"hello\\\"world\"")?,
        "hello\"world\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"hello\\'world\"")?,
        "hello\\'world\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"hello\\world\"")?,
        "hello\\world\n"
    );

    // Test mixed quotes
    assert_eq!(run_and_get_stdout("echo 'hello'\"world\"")?, "helloworld\n");
    assert_eq!(run_and_get_stdout("echo \"hello\"'world'")?, "helloworld\n");

    // Test complex quotes
    assert_eq!(
        run_and_get_stdout("echo 'script\\\"worldtest\\\"shell'")?,
        "script\\\"worldtest\\\"shell\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"script\\\"worldtest\\\"shell\"")?,
        "script\"worldtest\"shell\n"
    );

    assert_eq!(
        run_and_get_stdout("echo 'script\\\"worldtest\\\"shell'")?,
        "script\\\"worldtest\\\"shell\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"script\\\"worldtest\\\"shell\"")?,
        "script\"worldtest\"shell\n"
    );

    // Test unclosed quotes (should error)
    let output = run_shell_command("echo 'hello")?;
    assert!(!output.status.success());

    Ok(())
}

#[test]
fn test_escape_handling() -> Result<()> {
    // Test backslash escaping
    assert_eq!(run_and_get_stdout("echo hello\\ world")?, "hello world\n");
    assert_eq!(run_and_get_stdout("echo hello\\\\world")?, "hello\\world\n");
    assert_eq!(run_and_get_stdout("echo hello\\nworld")?, "hellonworld\n");

    // Test backslash in quotes
    assert_eq!(
        run_and_get_stdout("echo 'hello\\nworld'")?,
        "hello\\nworld\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"hello\\nworld\"")?,
        "hellonworld\n"
    );

    Ok(())
}

#[test]
fn test_token_handling() -> Result<()> {
    // Test multiple tokens
    assert_eq!(run_and_get_stdout("echo hello world")?, "hello world\n");
    assert_eq!(run_and_get_stdout("echo 'hello world'")?, "hello world\n");
    assert_eq!(run_and_get_stdout("echo \"hello world\"")?, "hello world\n");

    // Test tokens with spaces
    assert_eq!(run_and_get_stdout("echo 'hello  world'")?, "hello  world\n");
    assert_eq!(
        run_and_get_stdout("echo \"hello  world\"")?,
        "hello  world\n"
    );

    // Test tokens with special characters
    assert_eq!(run_and_get_stdout("echo 'hello;world'")?, "hello;world\n");
    assert_eq!(run_and_get_stdout("echo \"hello;world\"")?, "hello;world\n");

    Ok(())
}

#[test]
fn test_command_handling() -> Result<()> {
    // Test builtin commands
    assert_eq!(run_and_get_stdout("echo hello")?, "hello\n");
    assert!(!run_and_get_stdout("pwd")?.trim().is_empty());

    // Test exit command
    let output = run_shell_command("exit 42")?;
    assert_eq!(output.status.code(), Some(42));

    // Test unknown command
    let output = run_shell_command("nonexistentcommand")?;
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)?.contains("command not found"));

    Ok(())
}

#[test]
fn test_value_handling() -> Result<()> {
    // Test numeric values
    assert_eq!(run_and_get_stdout("echo 42")?, "42\n");
    assert_eq!(run_and_get_stdout("echo 3.14")?, "3.14\n");

    // Test array values
    assert_eq!(run_and_get_stdout("echo 1 2 3")?, "1 2 3\n");
    assert_eq!(run_and_get_stdout("echo '1 2' 3")?, "1 2 3\n");

    // Test mixed values
    assert_eq!(
        run_and_get_stdout("echo 42 'hello' 3.14")?,
        "42 hello 3.14\n"
    );
    assert_eq!(
        run_and_get_stdout("echo \"42\" 'hello' 3.14")?,
        "42 hello 3.14\n"
    );

    Ok(())
}
