/// Async reconfiguration state management.
module aptos_framework::reconfiguration_with_dkg {
    use std::features;
    use aptos_std::debug;
    use aptos_framework::consensus_config;
    use aptos_framework::dkg;
    use aptos_framework::execution_config;
    use aptos_framework::gas_schedule;
    use aptos_framework::jwk_consensus_config;
    use aptos_framework::jwks;
    use aptos_framework::keyless_account;
    use aptos_framework::mpc;
    use aptos_framework::randomness_api_v0_config;
    use aptos_framework::randomness_config;
    use aptos_framework::randomness_config_seqnum;
    use aptos_framework::reconfiguration;
    use aptos_framework::reconfiguration_state;
    use aptos_framework::stake;
    use aptos_framework::system_addresses;
    friend aptos_framework::block;
    friend aptos_framework::aptos_governance;

    /// Trigger a reconfiguration with DKG.
    /// Do nothing if one is already in progress.
    public(friend) fun try_start() {
        debug::print(&26);
        if (!reconfiguration_state::is_in_progress()) {
            debug::print(&28);
            // stake::finalize_next_validator_set();
            debug::print(&30);
            reconfiguration_state::on_reconfig_start();
            dkg::on_async_reconfig_start();
            mpc::on_async_reconfig_start();
            debug::print(&34);
        };
        debug::print(&36);
    }

    /// Clear incomplete DKG session, if it exists.
    /// Apply buffered on-chain configs (except for ValidatorSet, which is done inside `reconfiguration::reconfigure()`).
    /// Re-enable validator set changes.
    /// Run the default reconfiguration to enter the new epoch.
    public(friend) fun finish(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        dkg::try_clear_incomplete_session(framework);
        mpc::on_new_epoch(framework);

        // Apply buffered config changes.
        consensus_config::on_new_epoch(framework);
        execution_config::on_new_epoch(framework);
        gas_schedule::on_new_epoch(framework);
        std::version::on_new_epoch(framework);
        features::on_new_epoch(framework);
        jwk_consensus_config::on_new_epoch(framework);
        jwks::on_new_epoch(framework);
        keyless_account::on_new_epoch(framework);
        randomness_config_seqnum::on_new_epoch(framework);
        randomness_config::on_new_epoch(framework);
        randomness_api_v0_config::on_new_epoch(framework);
        reconfiguration::reconfigure();
    }

    /// Complete the current reconfiguration with DKG if possible.
    public(friend) fun try_finish(account: &signer) {
        let ready_for_next_epoch = true;
        ready_for_next_epoch = ready_for_next_epoch && dkg::ready_for_next_epoch();
        ready_for_next_epoch = ready_for_next_epoch && mpc::ready_for_next_epoch();
        if (ready_for_next_epoch) {
            finish(account);
        }
    }

    /// Complete the current reconfiguration with DKG.
    ///
    /// Abort if no DKG is in progress.
    ///
    /// Used only when feature `RECONFIG_REFACTORING` is not enabled.
    fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
        dkg::finish(dkg_result);
        finish(account);
    }
}
