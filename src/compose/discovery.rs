use anyhow::Result;
use glob::glob;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Find all compose files recursively from the given directory.
/// Filters out filenames containing prod/staging/production.
pub fn find_compose_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let patterns = [
        "compose*.yml",
        "compose*.yaml",
        "docker-compose*.yml",
        "docker-compose*.yaml",
        "**/compose*.yml",
        "**/compose*.yaml",
        "**/docker-compose*.yml",
        "**/docker-compose*.yaml",
    ];

    let excluded = ["prod", "staging", "production"];

    let mut found = BTreeSet::new();

    for pattern in &patterns {
        let full_pattern = dir.join(pattern).to_string_lossy().to_string();
        for entry in glob(&full_pattern)? {
            let path = entry?;
            if path.is_file() {
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                let dominated = excluded.iter().any(|ex| filename.contains(ex));
                if !dominated {
                    found.insert(path.canonicalize().unwrap_or(path));
                }
            }
        }
    }

    let mut result: Vec<PathBuf> = found.into_iter().collect();
    result.sort();
    Ok(result)
}
