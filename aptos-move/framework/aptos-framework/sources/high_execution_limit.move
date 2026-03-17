// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Manages the per-epoch slot counter for high-execution-limit transactions.
///
/// A fixed number of transactions per epoch may opt into a high-limit gas tier
/// by paying a flat premium. Higher limits are allocated on the first-come-first-served
/// basis. Whether transaction succeeds or not, the extended compute limit is considered
/// to be used.
module aptos_framework::high_execution_limit {
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_framework::system_addresses;
    use std::features;

    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;
    friend aptos_framework::reconfiguration;

    struct HighExecutionLimitConfig has key {
        /// Counter to track how many transactions can still use higher execution
        /// limits in this epoch.
        available: Aggregator<u64>,
        /// Maximum number of allowed high-limit transactions (per epoch).
        max_per_epoch: u64,
    }

    /// Called once during genesis or governance to install the resource.
    public fun initialize(aptos_framework: &signer, max_per_epoch: u64) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<HighExecutionLimitConfig>(@aptos_framework)) {
            move_to(aptos_framework, HighExecutionLimitConfig {
                available: aggregator_v2::create_aggregator_with_value(max_per_epoch, max_per_epoch),
                max_per_epoch,
            });
        }
    }

    /// For governance to update maximum number of allowed high-limit transactions per epoch.
    public fun update_max_per_epoch(aptos_framework: &signer, max_per_epoch: u64) acquires HighExecutionLimitConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (features::is_high_execution_limit_transactions_enabled()) {
            let config = borrow_global_mut<HighExecutionLimitConfig>(@aptos_framework);
            config.available = aggregator_v2::create_aggregator_with_value(max_per_epoch, max_per_epoch);
            config.max_per_epoch = max_per_epoch;
        }
    }

    /// Called at each epoch boundary to reset the counter.
    public(friend) fun on_new_epoch() acquires HighExecutionLimitConfig {
        let config = borrow_global_mut<HighExecutionLimitConfig>(@aptos_framework);
        let max_per_epoch = config.max_per_epoch;
        config.available = aggregator_v2::create_aggregator_with_value(max_per_epoch, max_per_epoch);
    }

    /// Returns true if the high execution limit is available. Only called in prologue.
    public(friend) fun is_high_execution_limit_available(): bool acquires HighExecutionLimitConfig {
        let config = borrow_global<HighExecutionLimitConfig>(@aptos_framework);
        config.available.is_at_least(1)
    }

    /// Decrements the counter marking high-execution limit as used. Only called in epilogue.
    ///
    /// # Precondition
    ///
    /// Prologue must check that the high execution limit is available.
    public(friend) fun record_used_high_execution_limit() acquires HighExecutionLimitConfig {
        let config = borrow_global_mut<HighExecutionLimitConfig>(@aptos_framework);
        config.available.sub(1);
    }
}
