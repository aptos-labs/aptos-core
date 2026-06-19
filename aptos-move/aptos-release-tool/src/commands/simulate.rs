// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `simulate`: run governance proposal simulation against a live network.
//!
//! This is a thin wrapper over `aptos-release-builder`'s simulation, which walks
//! any directory tree for `*.move` scripts — so it works directly on a bundle's
//! `scripts/` directory (or the bundle root).

use crate::network::NetworkSelection;
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

    aptos_release_builder::simulate::simulate_all_proposals(
        network.to_url()?,
        bundle_path,
        profile_gas,
        node_api_key,
    )
    .await
}
