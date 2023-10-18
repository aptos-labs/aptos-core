module aptos_framework::reconfiguration_v2 {
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

    public(friend) fun start() {
        let cur_epoch = reconfiguration::current_epoch();
        dkg::start(cur_epoch, stake::cur_validator_set(), cur_epoch + 1, stake::next_validator_set());
    }

    /// Apply buffered on-chain configs.
    /// Re-enable on-chain config changes.
    /// Trigger the default reconfiguration.
    public(friend) fun reconfigure(account: &signer) {
        features::on_new_epoch(account);
        consensus_config::on_new_epoch(account);
        execution_config::on_new_epoch(account);
        gas_schedule::on_new_epoch(account);
        std::version::on_new_epoch(account);
        config_for_next_epoch::enable_upserts(account);
        reconfiguration::reconfigure();
    }
}
