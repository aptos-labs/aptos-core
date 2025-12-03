script {
    use aptos_framework::aptos_governance;
    use aptos_framework::staking_config;

    fun update_recurring_lockup_duration(core_resources: &signer, new_recurring_lockup_duration_secs: u64) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        staking_config::update_recurring_lockup_duration_secs(&core_signer, new_recurring_lockup_duration_secs);
    }
}