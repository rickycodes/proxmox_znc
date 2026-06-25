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

pub fn detect_template_storages<R: CommandRunner>(runner: &R) -> Result<Vec<String>, String> {
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

        if !storages
            .iter()
            .any(|entry: &StorageEntry| entry.name == name)
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::CommandRunner;

    struct MockRunner {
        output: String,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, String> {
            if program == "pvesm" && args == ["status"] {
                Ok(self.output.clone())
            } else {
                Err("unexpected command".into())
            }
        }

        fn run_status(&self, _: &str, _: &[&str]) -> Result<(), String> {
            Err("unexpected command".into())
        }

        fn run_owned(&self, _: &str, _: &[String]) -> Result<String, String> {
            Err("unexpected command".into())
        }

        fn run_status_owned(&self, _: &str, _: &[String]) -> Result<(), String> {
            Err("unexpected command".into())
        }

        fn run_status_owned_with_input(
            &self,
            _: &str,
            _: &[String],
            _: &str,
        ) -> Result<(), String> {
            Err("unexpected command".into())
        }
    }

    #[test]
    fn detects_template_capable_storages() {
        let runner = MockRunner {
            output: "Name Type Status Total Used Available\nlocal dir active 100 1 99\nlocal-lvm lvmthin active 200 2 198\nwd nfs active 300 3 297\n".into(),
        };

        let storages = detect_template_storages(&runner).unwrap();
        assert_eq!(storages, vec!["local".to_string(), "wd".to_string()]);
    }

    #[test]
    fn template_support_list_includes_expected_types() {
        for kind in ["dir", "nfs", "cifs", "cephfs", "glusterfs", "zfs"] {
            assert!(supports_templates(kind), "{kind} should support templates");
        }
        assert!(!supports_templates("lvmthin"));
    }
}
