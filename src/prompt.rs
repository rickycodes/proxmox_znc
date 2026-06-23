use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::process::Command;

fn tty_pair() -> Result<(BufReader<std::fs::File>, std::fs::File), String> {
    let read = OpenOptions::new()
        .read(true)
        .open("/dev/tty")
        .map_err(|e| e.to_string())?;
    let write = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map_err(|e| e.to_string())?;
    Ok((BufReader::new(read), write))
}

pub fn text(prompt: &str, default: &str, slot: &mut Option<String>) -> Result<(), String> {
    if slot.as_deref().is_some() {
        return Ok(());
    }

    let (mut input, mut output) = tty_pair()?;
    write!(output, "{prompt} [{default}]: ").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut line = String::new();
    input.read_line(&mut line).map_err(|e| e.to_string())?;
    let line = line.trim_end_matches(['\n', '\r']);
    let value = if line.is_empty() { default } else { line };
    *slot = Some(value.to_string());
    Ok(())
}

pub fn number<T>(prompt: &str, default: T, slot: &mut Option<T>) -> Result<(), String>
where
    T: std::str::FromStr + ToString + Copy,
{
    if slot.is_some() {
        return Ok(());
    }

    let (mut input, mut output) = tty_pair()?;
    write!(output, "{prompt} [{}]: ", default.to_string()).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut line = String::new();
    input.read_line(&mut line).map_err(|e| e.to_string())?;
    let line = line.trim_end_matches(['\n', '\r']);
    *slot = Some(if line.is_empty() {
        default
    } else {
        line.parse().map_err(|_| format!("invalid number: {line}"))?
    });
    Ok(())
}

pub fn secret(prompt: &str, slot: &mut Option<String>) -> Result<(), String> {
    if slot.as_deref().is_some() {
        return Ok(());
    }

    let _ = Command::new("sh")
        .args(["-lc", "stty -echo < /dev/tty >/dev/tty 2>/dev/null"])
        .status();

    let (mut input, mut output) = tty_pair()?;
    write!(output, "{prompt}: ").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut line = String::new();
    input.read_line(&mut line).map_err(|e| e.to_string())?;
    let line = line.trim_end_matches(['\n', '\r']);

    let _ = Command::new("sh")
        .args(["-lc", "stty echo < /dev/tty >/dev/tty 2>/dev/null"])
        .status();
    let _ = writeln!(io::stdout());

    if line.is_empty() {
        return Err(format!("{prompt} is required"));
    }
    *slot = Some(line.to_string());
    Ok(())
}
