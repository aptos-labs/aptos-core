/// Reconfiguration with DKG helper functions.
module aptos_framework::reconfiguration_with_dkg {
    use std::error;
    use std::features;
    use std::signer;
    use aptos_framework::consensus_config;
    use aptos_framework::dkg;
    use aptos_framework::execution_config;
    use aptos_framework::gas_schedule;
    use aptos_framework::reconfiguration;
    use aptos_framework::stake;
    use aptos_framework::system_addresses;
    friend aptos_framework::block;
    friend aptos_framework::aptos_governance;

    const EPERMISSION_DENIED: u64 = 1;

    /// Trigger a reconfiguration with DKG.
    /// Do nothing if one is already in progress.
    public(friend) fun try_start(account: &signer) {
        assert!(system_addresses::is_reserved_address(signer::address_of(account)), error::invalid_argument(EPERMISSION_DENIED));
        if (dkg::in_progress()) { return };
        let cur_epoch = reconfiguration::current_epoch();
        stake::on_reconfig_start(account);
        dkg::start(cur_epoch, stake::cur_validator_set(), cur_epoch + 1, stake::new_validator_set(account));
    }

    /// Apply buffered on-chain configs (except for ValidatorSet, which is done inside `reconfiguration::reconfigure()`).
    /// Re-enable validator set changes.
    /// Run the default reconfiguration to enter the new epoch.
    public(friend) fun finish(account: &signer) {
        consensus_config::on_new_epoch(account);
        execution_config::on_new_epoch(account);
        gas_schedule::on_new_epoch(account);
        std::version::on_new_epoch(account);
        features::on_new_epoch(account);
        reconfiguration::reconfigure(account);
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
