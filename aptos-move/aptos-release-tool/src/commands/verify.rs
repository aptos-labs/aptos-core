// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `verify-bundle`: validate that a bundle is internally self-consistent.

use crate::{bundle, commands::combine_errors, config::BundleConfig, summary};
use anyhow::Result;
use std::{
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

/// Check the single-proposal layout: a top-level `metadata.json` and a
/// non-empty `scripts/` directory.
fn check_layout(bundle_path: &Path, errors: &mut Vec<String>) {
    if !bundle_path.join(bundle::METADATA_JSON).is_file() {
        errors.push(format!("missing {}", bundle::METADATA_JSON));
    }

    let scripts_dir = bundle_path.join(bundle::SCRIPTS_DIR);
    let has_move = fs::read_dir(&scripts_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.path().extension().map(|x| x == "move").unwrap_or(false))
        })
        .unwrap_or(false);
    if !has_move {
        errors.push(format!("{}/ has no .move scripts", bundle::SCRIPTS_DIR));
    }
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
