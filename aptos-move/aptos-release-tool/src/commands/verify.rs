// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `verify-bundle`: validate that a bundle is internally self-consistent.

use crate::{bundle, commands::combine_errors, config::BundleConfig, summary};
use anyhow::Result;
use aptos_crypto::HashValue;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn run(bundle_path: &Path, require_signoff: bool) -> Result<()> {
    let mut errors: Vec<String> = vec![];

    let manifest = bundle::BundleManifest::read(bundle_path)?;

    let config_yaml = bundle_path.join(bundle::CONFIG_YAML);
    let config = match BundleConfig::load(&config_yaml) {
        Ok(c) => Some(c),
        Err(e) => {
            errors.push(format!("failed to load {}: {}", config_yaml.display(), e));
            None
        },
    };

    // 1. Checksums: every file matches the manifest, with none missing or extra.
    match bundle::verify_checksums(bundle_path, &manifest.checksums) {
        Ok(checksum_errors) => {
            for p in checksum_errors {
                errors.push(p.to_string());
            }
        },
        Err(e) => errors.push(format!("failed to verify checksums: {}", e)),
    }

    // 1b. Global digest: the recorded digest must match the manifest's content.
    let computed = manifest.compute_digest();
    if computed != manifest.integrity.digest {
        errors.push(format!(
            "bundle digest mismatch:\n      recorded {}\n      computed {}",
            manifest.integrity.digest, computed
        ));
    }

    // 2. Consistency between bundle.toml and config.yaml, plus layout.
    if let Some(config) = &config
        && manifest.bundle.name != config.name
    {
        errors.push(format!(
            "bundle.toml name ({}) does not match config.yaml name ({})",
            manifest.bundle.name, config.name
        ));
    }
    check_layout(bundle_path, &mut errors);

    // 3. Optional sign-off enforcement.
    if require_signoff {
        check_signoff(bundle_path, &mut errors);
    }

    // Informational: report which summaries have been signed off.
    report_signoff_info(bundle_path);

    if errors.is_empty() {
        println!("verify-bundle: OK ({})", bundle_path.display());
        Ok(())
    } else {
        Err(combine_errors("verify-bundle", &errors))
    }
}

/// Check the single-proposal layout: a top-level `metadata.json`, plus the
/// script sources and compiled bytecode (see [`check_scripts`]).
fn check_layout(bundle_path: &Path, errors: &mut Vec<String>) {
    if !bundle_path.join(bundle::METADATA_JSON).is_file() {
        errors.push(format!("missing {}", bundle::METADATA_JSON));
    }
    check_scripts(bundle_path, errors);
}

/// Check that `scripts/` sources and `bytecode/` blobs pair 1:1 by file stem,
/// and that each blob's hash matches the execution hash stamped on its source
/// -- so what was audited (the stamped source) is what gets deployed (the
/// blob), without needing a compiler.
fn check_scripts(bundle_path: &Path, errors: &mut Vec<String>) {
    let sources = files_by_stem(&bundle_path.join(bundle::SCRIPTS_DIR), "move");
    let blobs = files_by_stem(&bundle_path.join(bundle::BYTECODE_DIR), "mv");
    if sources.is_empty() {
        errors.push(format!("{}/ has no .move scripts", bundle::SCRIPTS_DIR));
    }
    if blobs.is_empty() {
        errors.push(format!(
            "{}/ has no compiled .mv scripts",
            bundle::BYTECODE_DIR
        ));
    }

    for (stem, source_path) in &sources {
        let Some(blob_path) = blobs.get(stem) else {
            errors.push(format!(
                "{}/{}.move has no compiled counterpart {}/{}.mv",
                bundle::SCRIPTS_DIR,
                stem,
                bundle::BYTECODE_DIR,
                stem
            ));
            continue;
        };
        let source = match fs::read_to_string(source_path) {
            Ok(source) => source,
            Err(e) => {
                errors.push(format!("failed to read {}: {}", source_path.display(), e));
                continue;
            },
        };
        let blob = match fs::read(blob_path) {
            Ok(blob) => blob,
            Err(e) => {
                errors.push(format!("failed to read {}: {}", blob_path.display(), e));
                continue;
            },
        };
        match stamped_hash(&source) {
            None => errors.push(format!(
                "{}/{}.move has no stamped execution hash (expected a leading \
                 '// Script hash: ...' comment, added by generate-bundle)",
                bundle::SCRIPTS_DIR,
                stem
            )),
            Some(stamped) => {
                let actual = HashValue::sha3_256_of(&blob).to_hex().to_lowercase();
                if actual != stamped {
                    errors.push(format!(
                        "{}/{}.mv hash {} does not match the hash stamped on its source {}",
                        bundle::BYTECODE_DIR,
                        stem,
                        actual,
                        stamped
                    ));
                }
            },
        }
    }
    for stem in blobs.keys() {
        if !sources.contains_key(stem) {
            errors.push(format!(
                "{}/{}.mv has no source counterpart {}/{}.move",
                bundle::BYTECODE_DIR,
                stem,
                bundle::SCRIPTS_DIR,
                stem
            ));
        }
    }
}

/// The files with the given extension directly under `dir`, keyed by file
/// stem; an absent or unreadable directory yields an empty map.
fn files_by_stem(dir: &Path, extension: &str) -> BTreeMap<String, PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == extension).unwrap_or(false))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            Some((stem, p))
        })
        .collect()
}

/// The execution hash stamped as the script's first line by generate-bundle.
fn stamped_hash(source: &str) -> Option<String> {
    source
        .lines()
        .next()?
        .strip_prefix("// Script hash: ")
        .map(|s| s.trim().trim_start_matches("0x").to_lowercase())
}

/// Every `*.md` file under `summary/` as `(path, contents)`, sorted by path;
/// unreadable or absent files are skipped.
fn summary_files(bundle_path: &Path) -> Vec<(PathBuf, String)> {
    let summary_dir = bundle_path.join(bundle::SUMMARY_DIR);
    let Ok(entries) = fs::read_dir(&summary_dir) else {
        return vec![];
    };
    let mut files: Vec<(PathBuf, String)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .filter_map(|p| fs::read_to_string(&p).ok().map(|c| (p, c)))
        .collect();
    files.sort();
    files
}

/// Print each summary file's sign-off state (informational; box ticks are
/// checksum-neutral).
fn report_signoff_info(bundle_path: &Path) {
    for (path, contents) in summary_files(bundle_path) {
        let (ticked, total) = summary::box_counts(&contents);
        if total == 0 {
            continue;
        }
        let status = if ticked == total {
            "fully signed off"
        } else if ticked > 0 {
            "partially signed off"
        } else {
            "not signed off"
        };
        println!(
            "info: {} ({}/{}) in {}",
            status,
            ticked,
            total,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }
}

fn check_signoff(bundle_path: &Path, errors: &mut Vec<String>) {
    for (path, contents) in summary_files(bundle_path) {
        if summary::has_unchecked_boxes(&contents) {
            errors.push(format!(
                "sign-off required but {} has unchecked boxes",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stamped_hash;

    #[test]
    fn stamped_hash_parses_the_leading_comment() {
        let source = "// Script hash: 0xAB12ef\nscript { fun main() {} }";
        assert_eq!(stamped_hash(source), Some("ab12ef".to_string()));

        let unstamped = "script { fun main() {} }";
        assert_eq!(stamped_hash(unstamped), None);
    }
}
