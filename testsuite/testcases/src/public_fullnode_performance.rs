// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use anyhow::Error;
use aptos_forge::{NetworkContext, NetworkTest, Result, Test};
use aptos_logger::info;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_types::PeerId;
use tokio::runtime::Runtime;

/// A simple test that adds multiple public fullnodes (PFNs)
/// to the swarm and submits transactions through them.
pub struct PFNPerformance;

impl Test for PFNPerformance {
    fn name(&self) -> &'static str {
        "PFNPerformance"
    }
}

impl NetworkTest for PFNPerformance {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}

impl NetworkLoadTest for PFNPerformance {
    /// We must override the setup function to: (i) create PFNs in
    /// the swarm; and (ii) use those PFNs as the load destination.
    fn setup(&self, ctx: &mut NetworkContext) -> Result<LoadDestination> {
        // Add the PFNs to the swarm
        let num_pfns = 10;
        let pfn_peer_ids = create_and_add_pfns(ctx, num_pfns)?;

        // Use the PFNs as the load destination
        Ok(LoadDestination::Peers(pfn_peer_ids))
    }
}

/// Adds a number of PFNs to the network and returns the peer IDs
fn create_and_add_pfns(ctx: &mut NetworkContext, num_pfns: u64) -> Result<Vec<PeerId>, Error> {
    info!("Creating {} public fullnodes!", num_pfns);

    // Identify the version for the PFNs
    let swarm = ctx.swarm();
    let pfn_version = swarm.versions().max().unwrap();

    // Create the PFN swarm
    let runtime = Runtime::new().unwrap();
    let pfn_peer_ids: Vec<AccountAddress> = (0..num_pfns)
        .map(|_| {
            // Create a config for the PFN. Note: this needs to be done here
            // because the config will generate a unique peer ID for the PFN.
            let pfn_config = swarm.get_default_pfn_node_config();

            // Add the PFN to the swarm
            let peer_id = runtime
                .block_on(swarm.add_full_node(&pfn_version, pfn_config))
                .unwrap();

            // Verify the PFN was added
            if swarm.full_node(peer_id).is_none() {
                panic!(
                    "Failed to locate the PFN in the swarm! Peer ID: {:?}",
                    peer_id
                );
            }

            // Return the peer ID
            info!("Created new PFN with peer ID: {:?}", peer_id);
            peer_id
        })
        .collect();

    Ok(pfn_peer_ids)
}
