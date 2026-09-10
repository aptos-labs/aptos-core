// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `simulate`: run governance proposal simulation against a live network.
//!
//! Executes the bundle's compiled scripts (`bytecode/`) -- the exact bytes
//! deploy-testnet submits -- against a fork of the target network. The
//! bundle's script sources are never recompiled; they are audit material.

use crate::{bundle, network::NetworkSelection};
use anyhow::Result;
use std::path::Path;

pub async fn run(
    bundle_path: &Path,
    network: &NetworkSelection,
    profile_gas: bool,
    node_api_key: Option<String>,
) -> Result<()> {
    // Gate on the bundle's own integrity before simulating its scripts.
    crate::commands::verify::run(bundle_path, false)?;

    let scripts = bundle::load_compiled_scripts(bundle_path)?;

    // Keep gas reports outside the bundle: extra files would break its
    // checksums.
    let canonical = bundle_path
        .canonicalize()
        .unwrap_or_else(|_| bundle_path.to_path_buf());
    let gas_report_dir = canonical.with_file_name(format!(
        "{}-gas-profiling",
        canonical.file_name().unwrap_or_default().to_string_lossy()
    ));

    aptos_release_builder::simulate::simulate_compiled_scripts(
        network.to_url()?,
        &gas_report_dir,
        &scripts,
        profile_gas,
        node_api_key,
    )
    .await
}
