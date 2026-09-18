// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! What makes a bundle valid, behind a single [`verify`] entry point.

use crate::{
    compiled_script_paths, verify_checksums, BundleManifest, BYTECODE_DIR, METADATA_JSON,
    SCRIPTS_DIR, SUMMARY_DIR,
};
use anyhow::{bail, Result};
use sha3::{Digest, Sha3_256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const UNTICKED_BOX: &str = "[ ]";

/// Check that `bundle_path` holds a valid bundle, returning its manifest. The
/// error lists every problem found.
/// - Every file matches the manifest's checksums, and the digest matches.
/// - `metadata.json` and at least one compiled script are present.
/// - Every script source pairs with a compiled script and is stamped with its hash.
/// - Every sign-off checkbox is ticked, if `require_signoff`.
pub fn verify(bundle_path: &Path, require_signoff: bool) -> Result<BundleManifest> {
    let manifest = BundleManifest::read(bundle_path)?;

    let mut errors = integrity_errors(bundle_path, &manifest);
    errors.extend(layout_errors(bundle_path));
    errors.extend(source_errors(bundle_path));
    if require_signoff {
        errors.extend(signoff_errors(bundle_path));
    }

    if errors.is_empty() {
        Ok(manifest)
    } else {
        bail!(
            "bundle {} failed verification with {} error(s):\n  - {}",
            bundle_path.display(),
            errors.len(),
            errors.join("\n  - ")
        );
    }
}

/// Check the bundle's files against its manifest: a checksum mismatch, a
/// missing or extra file, or a digest mismatch.
fn integrity_errors(bundle_path: &Path, manifest: &BundleManifest) -> Vec<String> {
    let mut errors: Vec<String> = vec![];

    // 1. Checksums: every file matches the manifest, with none missing or extra.
    match verify_checksums(bundle_path, &manifest.checksums) {
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

    errors
}

/// Check the single-proposal layout: a top-level `metadata.json`, plus at
/// least one compiled script.
fn layout_errors(bundle_path: &Path) -> Vec<String> {
    let mut errors = vec![];
    if !bundle_path.join(METADATA_JSON).is_file() {
        errors.push(format!("missing {}", METADATA_JSON));
    }
    if compiled_script_paths(bundle_path).is_empty() {
        errors.push(format!("{}/ has no compiled .mv scripts", BYTECODE_DIR));
    }
    errors
}

/// Check that `scripts/` sources and `bytecode/` blobs pair 1:1 by file stem,
/// and that each blob's hash matches the execution hash stamped on its source
/// -- so what was audited (the stamped source) is what gets deployed (the
/// blob), without needing a compiler.
fn source_errors(bundle_path: &Path) -> Vec<String> {
    let mut errors = vec![];
    let sources = files_by_stem(&bundle_path.join(SCRIPTS_DIR), "move");
    let blobs = files_by_stem(&bundle_path.join(BYTECODE_DIR), "mv");
    if sources.is_empty() {
        errors.push(format!("{}/ has no .move scripts", SCRIPTS_DIR));
    }

    for (stem, source_path) in &sources {
        let Some(blob_path) = blobs.get(stem) else {
            errors.push(format!(
                "{}/{}.move has no compiled counterpart {}/{}.mv",
                SCRIPTS_DIR, stem, BYTECODE_DIR, stem
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
                SCRIPTS_DIR, stem
            )),
            Some(stamped) => {
                let actual = hex::encode(Sha3_256::digest(&blob));
                if actual != stamped {
                    errors.push(format!(
                        "{}/{}.mv hash {} does not match the hash stamped on its source {}",
                        BYTECODE_DIR, stem, actual, stamped
                    ));
                }
            },
        }
    }
    for stem in blobs.keys() {
        if !sources.contains_key(stem) {
            errors.push(format!(
                "{}/{}.mv has no source counterpart {}/{}.move",
                BYTECODE_DIR, stem, SCRIPTS_DIR, stem
            ));
        }
    }
    errors
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
    let summary_dir = bundle_path.join(SUMMARY_DIR);
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

/// The sign-off state of each summary file as `(path, ticked, total)`
/// checkboxes, sorted by path.
pub fn signoff_status(bundle_path: &Path) -> Vec<(PathBuf, usize, usize)> {
    summary_files(bundle_path)
        .into_iter()
        .map(|(path, contents)| {
            let ticked = contents.matches("[x]").count() + contents.matches("[X]").count();
            let unticked = contents.matches(UNTICKED_BOX).count();
            (path, ticked, ticked + unticked)
        })
        .collect()
}

/// One message per summary file that still has an unticked checkbox.
fn signoff_errors(bundle_path: &Path) -> Vec<String> {
    let mut errors = vec![];
    for (path, contents) in summary_files(bundle_path) {
        if contents.contains(UNTICKED_BOX) {
            errors.push(format!(
                "sign-off required but {} has unchecked boxes",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    errors
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
