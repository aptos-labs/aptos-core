script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;
    use aptos_framework::staking_config;
    use std::features;
    use aptos_std::fixed_point64;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        governed_gas_pool::initialize_governed_gas_pool_extension(&core_signer);
        //TODO: we should relax the max to 1000M move, for minimal, we should update to 10M Move before official migration
        staking_config::update_required_stake(&core_signer, 1000, 100_000_000_000_000_000);

        staking_config::update_rewards_rate(&core_signer, 1_000_000, 100_000_000);
        let enabled_blob: vector<u64> = vector[
            224, // STAKE_REWARD_USING_TREASURY
        ];

        let disabled_blob: vector<u64> = vector[];
        features::change_feature_flags_for_next_epoch(&core_signer, enabled_blob, disabled_blob);

        // Update the staking reward to 10% APY
        // The rewards rate is per-epoch, not per-year
        // Formula: rewards_amount = stake * (rewards_rate / rewards_rate_denominator) * (successful_proposals / total_proposals)
        //
        // For 10% APY with 2-hour epochs (4,380 epochs per year):
        // Per-epoch rate = (1.10)^(1/4380) - 1 ≈ 0,000021761
        //
        // Using high precision: Reward_rate = 0,000021761 / 100_000_000 = 2176 per epoch
        // This compounds to approximately 10% APY over 4,380 epochs
        // For 2-hour epochs (4,380 per year) targeting 10% APY:
        // Check if the new periodical_reward_rate_decrease feature is enabled
        assert!(!features::periodical_reward_rate_decrease_enabled());
        staking_config::update_rewards_rate(&core_signer, 2176, 100_000_000);
        staking_config::update_voting_power_increase_limit(&core_signer, 50*1_000_000_000);

        aptos_governance::force_end_epoch(&core_signer);
    }
}
