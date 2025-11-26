script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;
    use aptos_framework::staking_config;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        block::update_epoch_interval_microsecs(&core_signer, 7_200_000_000); // 2hrs
        // Minimual stake is set to 10M Move, This can only be done when the validator has sufficient stake, Otherwise, they be be removed
        // staking_config::update_required_stake(&core_signer, 1_000_000_000_000_000, 100_000_000_000_000_000);
        staking_config::update_voting_power_increase_limit(&core_signer, 50);
        aptos_governance::force_end_epoch(&core_signer);
    }
}
