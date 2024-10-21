/// Formal async reconfiguration state management to replace `reconfiguration_with_dkg.move`.
///
/// Every feature that requires end-of-epoch processing now has to specify the following procedures.
/// - A function `on_async_reconfig_start()` to start the processing.
/// - A function `ready_for_next_epoch()` to inform the framework whether the feature needs more time for processing.
/// - A function  `on_new_epoch()` to clean things up right before epoch change.
module aptos_framework::async_reconfig {
    use std::features;
    use aptos_framework::consensus_config;
    use aptos_framework::dkg;
    use aptos_framework::execution_config;
    use aptos_framework::gas_schedule;
    use aptos_framework::jwk_consensus_config;
    use aptos_framework::jwks;
    use aptos_framework::keyless_account;
    use aptos_framework::randomness_api_v0_config;
    use aptos_framework::randomness_config;
    use aptos_framework::randomness_config_seqnum;
    use aptos_framework::reconfiguration;
    use aptos_framework::reconfiguration_state;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;

    /// Trigger an async reconfig. More specifically,
    /// - for every feature that requires end-of-epoch processing, call its `on_async_reconfig_start()` hook.
    ///
    /// Do nothing if one is already in progress.
    public(friend) fun try_start() {
        if (!reconfiguration_state::is_in_progress()) {
            reconfiguration_state::on_reconfig_start();
            dkg::on_async_reconfig_start();
            // another_feature::on_async_reconfig_start();
        };
    }

    ///
    public(friend) fun force_finish(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        dkg::on_new_epoch(framework);
        // another_feature::on_new_epoch(framework);

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
    public(friend) fun try_finish(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        let ready_for_next_epoch = true;
        ready_for_next_epoch = ready_for_next_epoch && dkg::ready_for_next_epoch();
        // ready_for_next_epoch = ready_for_next_epoch && another_feature::ready_for_next_epoch();
        if (ready_for_next_epoch) {
            force_finish(framework);
        }
    }
}
