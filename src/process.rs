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

fn format_failure(program: &str, status: std::process::ExitStatus, stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();

    let mut parts = vec![format!("{program} exited with status {status}")];
    if !stderr.is_empty() {
        parts.push(format!("stderr: {stderr}"));
    }
    if !stdout.is_empty() {
        parts.push(format!("stdout: {stdout}"));
    }

    parts.join(" | ")
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
            return Err(format_failure(
                program,
                output.status,
                &output.stdout,
                &output.stderr,
            ));
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
            return Err(format_failure(
                program,
                output.status,
                &output.stdout,
                &output.stderr,
            ));
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
            Err(format_failure(
                program,
                output.status,
                &output.stdout,
                &output.stderr,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_returns_stdout_on_success() {
        let runner = ShellRunner;
        let output = runner.run("sh", &["-lc", "printf hello"]).unwrap();
        assert_eq!(output, "hello");
    }

    #[test]
    fn run_includes_stdout_and_stderr_on_failure() {
        let runner = ShellRunner;
        let err = runner
            .run("sh", &["-lc", "printf out; printf err >&2; exit 1"])
            .unwrap_err();

        assert!(err.contains("exited with status"));
        assert!(err.contains("stderr: err"));
        assert!(err.contains("stdout: out"));
    }

    #[test]
    fn run_owned_includes_stdout_and_stderr_on_failure() {
        let runner = ShellRunner;
        let args = vec![String::from("-lc"), String::from("printf out; printf err >&2; exit 1")];
        let err = runner.run_owned("sh", &args).unwrap_err();

        assert!(err.contains("exited with status"));
        assert!(err.contains("stderr: err"));
        assert!(err.contains("stdout: out"));
    }
}
