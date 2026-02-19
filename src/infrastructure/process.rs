use std::process::{Command, Output};

pub(crate) fn stderr_trimmed(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

pub(crate) fn stderr_or_status(output: &Output) -> String {
    let stderr = stderr_trimmed(output);
    if !stderr.is_empty() {
        return stderr;
    }

    format!("exit status {}", output.status)
}

pub(crate) fn execute_command(command: &[String]) -> std::io::Result<()> {
    if command.is_empty() {
        return Ok(());
    }

    let output = Command::new(&command[0]).args(&command[1..]).output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "command failed: {}; {}",
        command.join(" "),
        stderr_or_status(&output),
    )))
}

#[cfg(test)]
mod tests;
