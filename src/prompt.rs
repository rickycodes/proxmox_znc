use std::io::{self, Write};
use std::process::Command;

pub fn text(prompt: &str, default: &str, slot: &mut Option<String>) -> Result<(), String> {
    if slot.as_deref().is_some() {
        return Ok(());
    }

    print!("{prompt} [{default}]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    let input = input.trim_end_matches(['\n', '\r']);
    let value = if input.is_empty() { default } else { input };
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

    print!("{prompt} [{}]: ", default.to_string());
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    let input = input.trim_end_matches(['\n', '\r']);
    *slot = Some(if input.is_empty() {
        default
    } else {
        input
            .parse()
            .map_err(|_| format!("invalid number: {input}"))?
    });
    Ok(())
}

pub fn secret(prompt: &str, slot: &mut Option<String>) -> Result<(), String> {
    if slot.as_deref().is_some() {
        return Ok(());
    }

    let _ = Command::new("sh")
        .args(["-lc", "stty -echo < /dev/tty >/dev/null 2>&1"])
        .status();

    print!("{prompt}: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    let input = input.trim_end_matches(['\n', '\r']);

    let _ = Command::new("sh")
        .args(["-lc", "stty echo < /dev/tty >/dev/null 2>&1"])
        .status();
    println!();

    if input.is_empty() {
        return Err(format!("{prompt} is required"));
    }
    *slot = Some(input.to_string());
    Ok(())
}
