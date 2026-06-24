use std::process::{Command, Stdio};

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, String>;
    fn run_status(&self, program: &str, args: &[&str]) -> Result<(), String>;
    fn run_owned(&self, program: &str, args: &[String]) -> Result<String, String>;
    fn run_status_owned(&self, program: &str, args: &[String]) -> Result<(), String>;
    fn run_status_owned_with_input(
        &self,
        program: &str,
        args: &[String],
        input: &str,
    ) -> Result<(), String>;
}

pub struct ShellRunner;

impl CommandRunner for ShellRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_status(&self, program: &str, args: &[&str]) -> Result<(), String> {
        let status = Command::new(program)
            .args(args)
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} exited with status {status}"))
        }
    }

    fn run_owned(&self, program: &str, args: &[String]) -> Result<String, String> {
        let output = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_status_owned(&self, program: &str, args: &[String]) -> Result<(), String> {
        let status = Command::new(program)
            .args(args)
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} exited with status {status}"))
        }
    }

    fn run_status_owned_with_input(
        &self,
        program: &str,
        args: &[String],
        input: &str,
    ) -> Result<(), String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
        }

        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}
