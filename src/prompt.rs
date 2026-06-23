use crate::constants;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::process::Command;

fn tty_pair() -> Result<(BufReader<std::fs::File>, std::fs::File), String> {
    let read = OpenOptions::new()
        .read(true)
        .open(constants::DEV_TTY)
        .map_err(|e| e.to_string())?;
    let write = OpenOptions::new()
        .write(true)
        .open(constants::DEV_TTY)
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

    let first = read_secret(prompt)?;
    let second = read_secret(&format!("{prompt} (confirm)"))?;

    if first != second {
        return Err(format!("{prompt} entries do not match"));
    }

    *slot = Some(first);
    Ok(())
}

pub fn choose(
    prompt: &str,
    default_index: usize,
    options: &[String],
    slot: &mut Option<String>,
) -> Result<(), String> {
    if slot.as_deref().is_some() {
        return Ok(());
    }

    if options.is_empty() {
        return Err(format!("{prompt}: no options available"));
    }

    let (mut input, mut output) = tty_pair()?;
    let default_index = default_index.min(options.len().saturating_sub(1));

    writeln!(output, "{prompt}:").map_err(|e| e.to_string())?;
    for (idx, option) in options.iter().enumerate() {
        writeln!(output, "  [{}] {}", idx + 1, option).map_err(|e| e.to_string())?;
    }

    loop {
        write!(output, "Choose [{}]: ", default_index + 1).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;

        let mut line = String::new();
        input.read_line(&mut line).map_err(|e| e.to_string())?;
        let line = line.trim_end_matches(['\n', '\r']);

        let idx = if line.is_empty() {
            default_index + 1
        } else {
            match line.parse::<usize>() {
                Ok(value) => value,
                Err(_) => {
                    writeln!(output, "Enter a number from 1 to {}", options.len())
                        .map_err(|e| e.to_string())?;
                    continue;
                }
            }
        };

        if idx == 0 || idx > options.len() {
            writeln!(output, "Enter a number from 1 to {}", options.len())
                .map_err(|e| e.to_string())?;
            continue;
        }

        *slot = Some(options[idx - 1].clone());
        return Ok(());
    }
}

fn read_secret(prompt: &str) -> Result<String, String> {
    let _ = Command::new("sh")
        .args([
            "-lc",
            &format!("stty -echo < {0} > {0} 2>/dev/null", constants::DEV_TTY),
        ])
        .status();

    let (mut input, mut output) = tty_pair()?;
    write!(output, "{prompt}: ").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut line = String::new();
    input.read_line(&mut line).map_err(|e| e.to_string())?;
    let line = line.trim_end_matches(['\n', '\r']).to_string();

    let _ = Command::new("sh")
        .args([
            "-lc",
            &format!("stty echo < {0} > {0} 2>/dev/null", constants::DEV_TTY),
        ])
        .status();
    let _ = writeln!(io::stdout());

    if line.is_empty() {
        return Err(format!("{prompt} is required"));
    }

    Ok(line)
}
