// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! UserPromptSubmit hook that detects the current Move package.
//!
//! Walks up from the current working directory looking for `Move.toml`.
//! When found, outputs JSON with `additionalContext` so the AI assistant
//! knows which package the user is working in. Outputs nothing if no
//! package is found. Always exits 0.

use crate::utilities::find_package_root;
use anyhow::Result;
use std::path::Path;

/// Entry point: detect the nearest Move package and emit context.
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    if let Some(pkg_path) = find_package_root(&cwd) {
        let manifest = pkg_path.join("Move.toml");
        let pkg_name = read_package_name(&manifest).unwrap_or_else(|| "(unknown)".to_string());
        let ctx = format!(
            "Current Move package: {} at {}.",
            pkg_name,
            pkg_path.display()
        );
        let output = serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
                "additionalContext": ctx
            }
        });
        println!("{}", output);
    }
    Ok(())
}

/// Read the package name from a `Move.toml` file by simple line scanning.
fn read_package_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            let value = trimmed.split_once('=')?.1.trim();
            // Strip surrounding quotes
            let name = value
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('\'')
                .trim_end_matches('\'');
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_package_name() {
        let dir = tempfile::tempdir().unwrap();
        let toml = dir.path().join("Move.toml");
        let mut f = std::fs::File::create(&toml).unwrap();
        writeln!(f, "[package]").unwrap();
        writeln!(f, "name = \"my_package\"").unwrap();
        assert_eq!(read_package_name(&toml), Some("my_package".to_string()));
    }

    #[test]
    fn test_read_package_name_single_quotes() {
        let dir = tempfile::tempdir().unwrap();
        let toml = dir.path().join("Move.toml");
        let mut f = std::fs::File::create(&toml).unwrap();
        writeln!(f, "[package]").unwrap();
        writeln!(f, "name = 'my_pkg'").unwrap();
        assert_eq!(read_package_name(&toml), Some("my_pkg".to_string()));
    }

    #[test]
    fn test_read_package_name_missing() {
        let dir = tempfile::tempdir().unwrap();
        let toml = dir.path().join("Move.toml");
        std::fs::write(&toml, "[dependencies]\n").unwrap();
        assert_eq!(read_package_name(&toml), None);
    }
}
