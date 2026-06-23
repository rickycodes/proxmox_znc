use crate::process::CommandRunner;

pub fn detect_storages<R: CommandRunner>(runner: &R) -> Result<Vec<String>, String> {
    let output = runner.run("pvesm", &["status"])?;
    let mut storages = Vec::new();

    for line in output.lines().skip(1) {
        let mut cols = line.split_whitespace();
        let name = cols.next().unwrap_or("");
        let _kind = cols.next().unwrap_or("");
        let state = cols.next().unwrap_or("");

        if name.is_empty() || state != "active" {
            continue;
        }

        if !storages.iter().any(|s| s == name) {
            storages.push(name.to_string());
        }
    }

    Ok(storages)
}
