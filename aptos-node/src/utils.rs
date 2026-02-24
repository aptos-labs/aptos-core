// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::anyhow;
use aptos_config::config::{NodeConfig, DEFAULT_EXECUTION_CONCURRENCY_LEVEL};
#[cfg(unix)]
use aptos_logger::prelude::*;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReaderWriter,
};
use aptos_types::{
    account_config::ChainIdResource, chain_id::ChainId, on_chain_config::OnChainConfig,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::prod_configs::{
    set_async_runtime_checks, set_layout_caches, set_paranoid_type_checks,
};

/// Error message to display when non-production features are enabled
pub const ERROR_MSG_BAD_FEATURE_FLAGS: &str = r#"
aptos-node was compiled with feature flags that shouldn't be enabled.

This is caused by cargo's feature unification.
When you compile two crates with a shared dependency, if one enables a feature flag for the dependency, then it is also enabled for the other crate.

To resolve this issue, try the following methods:
- Recompile `aptos-node` SEPARATELY
- Check if a disallowed feature flag is enabled by a dependency in the build tree
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
    set_layout_caches(node_config.execution.layout_caches_enabled);
    set_paranoid_type_checks(node_config.execution.paranoid_type_verification);
    set_async_runtime_checks(node_config.execution.async_runtime_checks);
    let effective_concurrency_level = if node_config.execution.concurrency_level == 0 {
        ((num_cpus::get() / 2) as u16).clamp(1, DEFAULT_EXECUTION_CONCURRENCY_LEVEL)
    } else {
        node_config.execution.concurrency_level
    };
    AptosVM::set_concurrency_level_once(effective_concurrency_level as usize);
    AptosVM::set_discard_failed_blocks(node_config.execution.discard_failed_blocks);
    AptosVM::set_num_proof_reading_threads_once(
        node_config.execution.num_proof_reading_threads as usize,
    );
    AptosVM::set_blockstm_v2_enabled_once(node_config.execution.blockstm_v2_enabled);
    AptosVM::set_enable_pre_write_once(node_config.execution.enable_pre_write);

    if node_config
        .execution
        .processed_transactions_detailed_counters
    {
        AptosVM::set_processed_transactions_detailed_counters();
    }
}

#[cfg(not(unix))]
pub fn ensure_max_open_files_limit(_required: u64, _assert_success: bool) {}

#[cfg(unix)]
pub fn ensure_max_open_files_limit(required: u64, assert_success: bool) {
    if required == 0 {
        return;
    }

    // Only works on Unix environments
    #[cfg(unix)]
    {
        if !rlimit::Resource::NOFILE.is_supported() {
            warn!(
                required = required,
                "rlimit setting not supported on this platform. Won't ensure."
            );
            return;
        }

        let (soft, mut hard) = match rlimit::Resource::NOFILE.get() {
            Ok((soft, hard)) => (soft, hard),
            Err(err) => {
                warn!(
                    error = ?err,
                    required = required,
                    "Failed getting RLIMIT_NOFILE. Won't ensure."
                );
                return;
            },
        };

        if soft >= required {
            return;
        }

        if required > hard {
            warn!(
                hard_limit = hard,
                required = required,
                "System RLIMIT_NOFILE hard limit too small."
            );
            // Not panicking right away -- user can be root
            hard = required;
        }

        rlimit::Resource::NOFILE
            .set(required, hard)
            .unwrap_or_else(|err| {
                let msg = format!("RLIMIT_NOFILE soft limit is {soft}, configured requirement is {required}, and \
                    failed to raise to it. Please make sure that `limit -n` shows a number larger than \
                    {required} before starting the node. Error: {err}.");
                if assert_success {
                    panic!("{}", msg)
                } else {
                    error!("{}", msg)
                }
            });
    }
}
