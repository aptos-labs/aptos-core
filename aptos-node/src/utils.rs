// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use aptos_config::config::{NodeConfig, DEFAULT_EXECUTION_CONCURRENCY_LEVEL};
use aptos_storage_interface::{state_view::LatestDbStateCheckpointView, DbReaderWriter};
use aptos_types::{
    account_config::ChainIdResource, chain_id::ChainId, on_chain_config::OnChainConfig,
    vm::configs::set_paranoid_type_checks,
};
use aptos_vm::AptosVM;
use std::cmp::min;

/// Error message to display when non-production features are enabled
pub const ERROR_MSG_BAD_FEATURE_FLAGS: &str = r#"
aptos-node was compiled with feature flags that shouldn't be enabled.

This is caused by cargo's feature unification.
When you compile two crates with a shared dependency, if one enables a feature flag for the dependency, then it is also enabled for the other crate.

PLEASE RECOMPILE APTOS-NODE SEPARATELY using the following command:
    cargo build --package aptos-node

"#;

/// Initializes a global rayon thread pool iff `create_global_rayon_pool` is true
pub fn create_global_rayon_pool(create_global_rayon_pool: bool) {
    if create_global_rayon_pool {
        rayon::ThreadPoolBuilder::new()
            .thread_name(|index| format!("rayon-global-{}", index))
            .build_global()
            .expect("Failed to build rayon global thread pool.");
    }
}

/// Fetches the chain ID from on-chain resources
pub fn fetch_chain_id(db: &DbReaderWriter) -> anyhow::Result<ChainId> {
    let db_state_view = db
        .reader
        .latest_state_checkpoint_view()
        .map_err(|err| anyhow!("[aptos-node] failed to create db state view {}", err))?;
    Ok(ChainIdResource::fetch_config(&db_state_view)
        .expect("[aptos-node] missing chain ID resource")
        .chain_id())
}

/// Sets the Aptos VM configuration based on the node configurations
pub fn set_aptos_vm_configurations(node_config: &NodeConfig) {
    set_paranoid_type_checks(node_config.execution.paranoid_type_verification);
    let effective_concurrency_level = if node_config.execution.concurrency_level == 0 {
        min(
            DEFAULT_EXECUTION_CONCURRENCY_LEVEL,
            (num_cpus::get() / 2) as u16,
        )
    } else {
        node_config.execution.concurrency_level
    };
    AptosVM::set_concurrency_level_once(effective_concurrency_level as usize);
    AptosVM::set_discard_failed_blocks(node_config.execution.discard_failed_blocks);
    AptosVM::set_num_proof_reading_threads_once(
        node_config.execution.num_proof_reading_threads as usize,
    );

    if node_config
        .execution
        .processed_transactions_detailed_counters
    {
        AptosVM::set_processed_transactions_detailed_counters();
    }
}
