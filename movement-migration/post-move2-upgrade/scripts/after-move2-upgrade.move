script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;
    use aptos_framework::staking_config;
    use std::features;
    use aptos_std::fixed_point64;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        governed_gas_pool::initialize_governed_gas_pool_extension(&core_signer);
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
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
        // Per-epoch rate = (1.10)^(1/4380) - 1 ≈ 0.0000217 or 0.00217%
        //
        // Using high precision: 22 / 1,000,000 = 0.0022% per epoch
        // This compounds to approximately 10% APY over 4,380 epochs
        // For 2-hour epochs (4,380 per year) targeting 10% APY:
        // Check if the new periodical_reward_rate_decrease feature is enabled
        assert!(!features::periodical_reward_rate_decrease_enabled());
        staking_config::update_rewards_rate(&core_signer, 22, 1_000_000);

        aptos_governance::force_end_epoch(&core_signer);
    }
}
