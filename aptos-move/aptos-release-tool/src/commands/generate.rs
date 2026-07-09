// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{bundle, config::BundleConfig, release, summary};
use anyhow::{Context, Result};
use aptos_release_builder::{components::get_execution_hash, ExecutionMode};
use aptos_types::on_chain_config::GasScheduleV2;
use chrono::Utc;
use std::{fs, path::Path};

/// The gas schedule snapshots materialized into the bundle from the proposal's
/// `Gas` entry.
struct GasArtifacts {
    old_sched: Option<GasScheduleV2>,
    new_sched: GasScheduleV2,
}

/// Produces a complete, self-contained governance bundle from a config.
pub async fn run(release_config_path: &Path, bundle_path: &Path, core_path: &Path) -> Result<()> {
    let config = BundleConfig::load(release_config_path)?;

    bundle::create_bundle_dir(bundle_path)?;

    // On failure, remove the partial bundle so the command stays retryable.
    let result = build_bundle(&config, release_config_path, bundle_path, core_path).await;
    if result.is_err() {
        let _ = fs::remove_dir_all(bundle_path);
    }
    result?;

    println!("\nBundle generated at {}", bundle_path.display());
    Ok(())
}

/// Build the bundle contents under the (already-created) `bundle_path`.
async fn build_bundle(
    config: &BundleConfig,
    release_config_path: &Path,
    bundle_path: &Path,
    core_path: &Path,
) -> Result<()> {
    // 1. Generate the proposal scripts and metadata.
    println!("Generating proposal scripts...");
    generate_scripts(config, bundle_path).await?;

    // 2. Materialize gas schedule snapshots.
    let gas = materialize_gas(config, bundle_path).await?;

    // 3. Generate human-reviewable summaries.
    println!("Writing summaries...");
    let summary_dir = bundle_path.join(bundle::SUMMARY_DIR);
    fs::create_dir_all(&summary_dir)?;
    if let Some(gas) = &gas {
        summary::write_gas_summary(&summary_dir, gas.old_sched.as_ref(), &gas.new_sched)?;
    }
    let (enabled, disabled) = release::collect_feature_changes(&config.update_sequence);
    summary::write_feature_flag_summary(&summary_dir, &enabled, &disabled)?;

    // 4. Copy the config verbatim into the bundle.
    fs::copy(release_config_path, bundle_path.join(bundle::CONFIG_YAML))
        .context("failed to copy config into bundle")?;

    // 5. Build and write the manifest (with checksums computed last).
    println!("Building manifest...");
    let source = bundle::read_source_info(core_path)?;
    let mut manifest = bundle::BundleManifest {
        format_version: bundle::BUNDLE_FORMAT_VERSION,
        bundle: bundle::BundleSection {
            name: config.name.clone(),
            created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        },
        source: bundle::SourceSection {
            branch: source.branch,
            commit: source.commit,
        },
        integrity: bundle::IntegritySection {
            digest: String::new(),
        },
        checksums: Default::default(),
    };
    manifest.checksums = bundle::compute_checksums(bundle_path)?;
    manifest.integrity.digest = manifest.compute_digest();
    manifest.write(bundle_path)?;

    // 6. Verify integrity before declaring success.
    println!("Verifying bundle integrity...");
    crate::commands::verify::run(bundle_path, false)?;
    Ok(())
}

/// Materialize the bundle's gas snapshots (`gas/{old,new}.json`) from the
/// proposal's `Gas` entry, or `None` if there is none or the change is a no-op
/// (`old == new`, for which no gas script is emitted).
async fn materialize_gas(
    config: &BundleConfig,
    bundle_path: &Path,
) -> Result<Option<GasArtifacts>> {
    let Some(gas) = release::find_gas_entry(&config.update_sequence) else {
        return Ok(None);
    };
    println!("Materializing gas schedule snapshots...");

    let new_sched = gas.new.fetch_gas_schedule().await?;
    let old_sched = match &gas.old {
        Some(old) => Some(old.fetch_gas_schedule().await?),
        None => None,
    };

    if let Some(old) = &old_sched
        && old == &new_sched
    {
        println!("Gas entry is a no-op (old == new); skipping gas artifacts.");
        return Ok(None);
    }

    let gas_dir = bundle_path.join(bundle::GAS_DIR);
    fs::create_dir_all(&gas_dir)?;
    fs::write(
        gas_dir.join("new.json"),
        serde_json::to_string_pretty(&new_sched)?,
    )?;
    if let Some(old) = &old_sched {
        fs::write(gas_dir.join("old.json"), serde_json::to_string_pretty(old)?)?;
    }

    Ok(Some(GasArtifacts {
        old_sched,
        new_sched,
    }))
}

/// Generate the proposal's multi-step governance scripts into `scripts/N-*.move`
/// and its metadata into top-level `metadata.json`.
async fn generate_scripts(config: &BundleConfig, bundle_path: &Path) -> Result<()> {
    let mut result: Vec<(String, String)> = vec![];
    for entry in config.update_sequence.iter().rev() {
        entry
            .generate_release_script(None, &mut result, ExecutionMode::MultiStep)
            .await?;
    }
    result.reverse();

    let scripts_dir = bundle_path.join(bundle::SCRIPTS_DIR);
    fs::create_dir_all(&scripts_dir)?;
    // Zero-pad the index so lexical order matches numeric step order at >= 10 scripts.
    let width = result.len().saturating_sub(1).to_string().len();
    for (idx, (script_name, script)) in result.iter().enumerate() {
        let file_name = format!("{:0width$}-{}.move", idx, script_name, width = width);
        let path = scripts_dir.join(&file_name);
        fs::write(&path, prepend_script_hash(script_name, script))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    let metadata_path = bundle_path.join(bundle::METADATA_JSON);
    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&config.metadata)?,
    )
    .with_context(|| format!("failed to write {}", metadata_path.display()))?;
    Ok(())
}

/// Prepend the script's on-chain execution hash as a comment (the hash
/// governance voters approve).
fn prepend_script_hash(script_name: &str, script: &str) -> String {
    let single = [(script_name.to_string(), script.to_string())];
    match get_execution_hash(&single) {
        Some(hash) => format!("// Script hash: {}\n{}", hash, script),
        None => script.to_string(),
    }
}
