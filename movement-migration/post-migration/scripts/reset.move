script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;
    use aptos_framework::staking_config;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        block::update_epoch_interval_microsecs(&core_signer, 7_200_000_000); // 2hrs
        // Minimual stake is set to 100K Move, This can only be done when the validator has sufficient stake, Otherwise, they be be removed
        staking_config::update_required_stake(&core_signer, 10_000_000_000_000, 100_000_000_000_000_000);
        staking_config::update_voting_power_increase_limit(&core_signer, 50);
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
        staking_config::update_rewards_rate(&core_signer, 2176, 100_000_000);
        aptos_governance::force_end_epoch(&core_signer);
    }
}
