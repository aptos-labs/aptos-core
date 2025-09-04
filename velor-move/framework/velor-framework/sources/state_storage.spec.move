spec velor_framework::state_storage {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Given the blockchain is in an operating state, the resources for tracking state storage usage and gas
    /// parameters must exist for the Velor framework address.
    /// Criticality: Critical
    /// Implementation: The initialize function ensures only the Velor framework address can call it.
    /// Enforcement: Formally verified via [high-level-req-1](module).
    ///
    /// No.: 2
    /// Requirement: During the initialization of the module, it is guaranteed that the resource for tracking state
    /// storage usage will be moved under the Velor framework account with default initial values.
    /// Criticality: Medium
    /// Implementation: The resource for tracking state storage usage may only be initialized with specific values and
    /// published under the velor_framework account.
    /// Enforcement: Formally verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: The initialization function is only called once, during genesis.
    /// Criticality: Medium
    /// Implementation: The initialize function ensures StateStorageUsage does not already exist.
    /// Enforcement: Formally verified via [high-level-req-3](initialize).
    ///
    /// No.: 4
    /// Requirement: During the initialization of the module, it is guaranteed that the resource for tracking state storage
    /// usage will be moved under the Velor framework account with default initial values.
    /// Criticality: Medium
    /// Implementation: The resource for tracking state storage usage may only be initialized with specific values and
    /// published under the velor_framework account.
    /// Enforcement: Formally verified via [high-level-req-4](initialize).
    ///
    /// No.: 5
    /// Requirement: The structure for tracking state storage usage should exist for it to be updated at the beginning of
    /// each new block and for retrieving the values of structure members.
    /// Criticality: Medium
    /// Implementation: The functions on_new_block and current_items_and_bytes verify that the StateStorageUsage
    /// structure exists before performing any further operations.
    /// Enforcement: Formally Verified via [high-level-req-5.1](current_items_and_bytes), [high-level-req-5.2](on_new_block), and the [high-level-req-5.3](global invariant).
    /// </high-level-req>
    ///
    spec module {
        use velor_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        /// [high-level-req-1]
        /// [high-level-req-5.3]
        invariant [suspendable] chain_status::is_operating() ==> exists<StateStorageUsage>(@velor_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<GasParameter>(@velor_framework);
    }

    /// ensure caller is admin.
    /// aborts if StateStorageUsage already exists.
    spec initialize(velor_framework: &signer) {
        use std::signer;
        let addr = signer::address_of(velor_framework);
        /// [high-level-req-4]
        aborts_if !system_addresses::is_velor_framework_address(addr);
        /// [high-level-req-3]
        aborts_if exists<StateStorageUsage>(@velor_framework);
        ensures exists<StateStorageUsage>(@velor_framework);
        let post state_usage = global<StateStorageUsage>(@velor_framework);
        /// [high-level-req-2]
        ensures state_usage.epoch == 0 && state_usage.usage.bytes == 0 && state_usage.usage.items == 0;
    }

    spec on_new_block(epoch: u64) {
        use velor_framework::chain_status;
        /// [high-level-req-5.2]
        requires chain_status::is_operating();
        aborts_if false;
        ensures epoch == global<StateStorageUsage>(@velor_framework).epoch;
    }

    spec current_items_and_bytes(): (u64, u64) {
        /// [high-level-req-5.1]
        aborts_if !exists<StateStorageUsage>(@velor_framework);
    }

    spec get_state_storage_usage_only_at_epoch_beginning(): Usage {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec on_reconfig {
        aborts_if true;
    }
}
