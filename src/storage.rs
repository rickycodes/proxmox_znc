use crate::constants;
use crate::process::CommandRunner;

#[derive(Debug, Clone)]
struct StorageEntry {
    name: String,
    kind: String,
}

pub fn detect_storages<R: CommandRunner>(runner: &R) -> Result<Vec<String>, String> {
    Ok(detect_storage_entries(runner)?
        .into_iter()
        .map(|entry| entry.name)
        .collect())
}

pub fn detect_template_storages<R: CommandRunner>(
    runner: &R,
) -> Result<Vec<String>, String> {
    let storages = detect_storage_entries(runner)?;
    let mut candidates = Vec::new();

    for storage in storages {
        if supports_templates(&storage.kind) && !candidates.iter().any(|s| s == &storage.name) {
            candidates.push(storage.name);
        }
    }

    Ok(candidates)
}

fn detect_storage_entries<R: CommandRunner>(runner: &R) -> Result<Vec<StorageEntry>, String> {
    let output = runner.run("pvesm", &["status"])?;
    let mut storages = Vec::new();

    for line in output.lines().skip(1) {
        let mut cols = line.split_whitespace();
        let name = cols.next().unwrap_or("");
        let kind = cols.next().unwrap_or("");
        let state = cols.next().unwrap_or("");

        if name.is_empty() || state != "active" {
            continue;
        }

        if !storages.iter().any(|entry: &StorageEntry| entry.name == name) {
            storages.push(StorageEntry {
                name: name.to_string(),
                kind: kind.to_string(),
            });
        }
    }

    Ok(storages)
}

fn supports_templates(kind: &str) -> bool {
    constants::TEMPLATE_STORAGE_KINDS.contains(&kind)
}
