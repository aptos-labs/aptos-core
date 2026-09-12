// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `verify-bundle`: validate that a bundle is internally self-consistent.

use crate::config::BundleConfig;
use anyhow::{bail, Context, Result};
use aptos_governance_bundle::{self as bundle, verify};
use std::path::Path;

pub fn run(bundle_path: &Path, require_signoff: bool) -> Result<()> {
    let manifest = verify::verify(bundle_path, require_signoff)?;

    // The config the bundle was generated from must load and agree with the manifest.
    // TODO: move this into `aptos_governance_bundle::verify` once the old release-builder code is
    // gone and the bundle config definition can live in that crate too.
    let config_yaml = bundle_path.join(bundle::CONFIG_YAML);
    let config = BundleConfig::load(&config_yaml)
        .with_context(|| format!("failed to load {}", config_yaml.display()))?;
    if manifest.bundle.name != config.name {
        bail!(
            "bundle.toml name ({}) does not match config.yaml name ({})",
            manifest.bundle.name,
            config.name
        );
    }

    report_signoff_info(bundle_path);
    println!("verify-bundle: OK ({})", bundle_path.display());
    Ok(())
}

/// Print each summary file's sign-off state (informational; box ticks are
/// checksum-neutral).
fn report_signoff_info(bundle_path: &Path) {
    for (path, ticked, total) in verify::signoff_status(bundle_path) {
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
