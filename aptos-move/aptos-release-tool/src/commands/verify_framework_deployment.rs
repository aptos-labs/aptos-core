// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `verify-framework-deployment`: check that a deployed framework release matches
//! its bundle by comparing on-chain state against it.
//!
//! TODO: this does not yet verify that the framework packages' bytecode was
//! actually published; for now we rely solely on the gas schedule change as the
//! signal that the release executed. Add a real framework-code check in the
//! future -- this may not be trivial and might require significant work.

use crate::{bundle, network::NetworkSelection};
use anyhow::{bail, Context, Result};
use aptos_release_builder::components::fetch_config;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::on_chain_config::GasScheduleV2;
use std::{fs, path::Path};

pub async fn run(
    bundle_path: &Path,
    network: &NetworkSelection,
    node_api_key: Option<String>,
) -> Result<()> {
    // Gate on the bundle's own integrity first, so we compare the chain against
    // trusted expected values.
    crate::commands::verify::run(bundle_path, false)?;

    let mut client = Client::builder(AptosBaseUrl::Custom(network.to_url()?));
    if let Some(key) = node_api_key.as_ref() {
        client = client.api_key(key)?;
    }
    let client = client.build();

    check_gas(bundle_path, &client)?;

    println!("\nverify-framework-deployment: OK");
    Ok(())
}

fn check_gas(bundle_path: &Path, client: &Client) -> Result<()> {
    let gas_new = bundle_path.join(bundle::GAS_DIR).join("new.json");
    if !gas_new.is_file() {
        bail!(
            "bundle has no gas schedule ({} missing); verify-framework-deployment relies \
             solely on the gas schedule to confirm the release, so there is nothing to check",
            gas_new.display()
        );
    }
    let contents = fs::read_to_string(&gas_new)
        .with_context(|| format!("failed to read {}", gas_new.display()))?;
    let expected: GasScheduleV2 = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", gas_new.display()))?;

    let on_chain =
        fetch_config::<GasScheduleV2>(client).context("failed to fetch on-chain gas schedule")?;

    if on_chain.feature_version != expected.feature_version {
        bail!(
            "gas feature version on-chain ({}) != bundle ({})",
            on_chain.feature_version,
            expected.feature_version
        );
    }
    let diff = GasScheduleV2::diff(&expected, &on_chain);
    if !diff.is_empty() {
        for name in diff.keys().take(20) {
            println!("      differs: {}", name);
        }
        bail!(
            "gas schedule differs from bundle in {} parameter(s)",
            diff.len()
        );
    }
    println!(
        "  gas schedule: MATCH (feature version {})",
        on_chain.feature_version
    );
    Ok(())
}
