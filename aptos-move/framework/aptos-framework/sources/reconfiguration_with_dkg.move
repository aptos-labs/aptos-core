/// Reconfiguration with DKG helper functions.
module aptos_framework::reconfiguration_with_dkg {
    use std::config_for_next_epoch;
    use std::features;
    use aptos_framework::consensus_config;
    use aptos_framework::dkg;
    use aptos_framework::execution_config;
    use aptos_framework::gas_schedule;
    use aptos_framework::reconfiguration;
    use aptos_framework::stake;
    friend aptos_framework::block;
    friend aptos_framework::aptos_governance;

    /// Trigger a reconfiguration with DKG.
    /// Abort if there is a DKG in progress.
    public(friend) fun start(account: &signer) {
        config_for_next_epoch::disable_upserts(account);
        let cur_epoch = reconfiguration::current_epoch();
        dkg::start(cur_epoch, stake::cur_validator_set(), cur_epoch + 1, stake::next_validator_set());
    }

    /// Apply buffered on-chain configs.
    /// Re-enable on-chain config changes.
    /// Trigger the default reconfiguration to enter the new epoch.
    public(friend) fun finish(account: &signer) {
        consensus_config::on_new_epoch(account);
        execution_config::on_new_epoch(account);
        gas_schedule::on_new_epoch(account);
        std::version::on_new_epoch(account);
        features::on_new_epoch(account);
        config_for_next_epoch::enable_upserts(account);
        reconfiguration::reconfigure();
    }

    /// Complete the current reconfiguration with DKG.
    /// Abort if no DKG is in progress.
    fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
        let should_finish = dkg::update(true, dkg_result);
        if (should_finish) {
            finish(account);
        }
    }
}
