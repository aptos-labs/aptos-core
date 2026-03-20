// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Write rendered files to the output directory, preserving relative paths.
pub fn write_output(output_dir: &Path, files: &[(PathBuf, String)]) -> Result<()> {
    for (rel_path, content) in files {
        let dest = output_dir.join(rel_path);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        std::fs::write(&dest, content)
            .with_context(|| format!("failed to write {}", dest.display()))?;

        println!("  wrote {}", rel_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_output_creates_files() {
        let dir = TempDir::new().unwrap();
        let files = vec![
            (PathBuf::from("a/b.txt"), "hello".to_string()),
            (PathBuf::from("c.txt"), "world".to_string()),
        ];

        write_output(dir.path(), &files).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("a/b.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("c.txt")).unwrap(),
            "world"
        );
    }
}
