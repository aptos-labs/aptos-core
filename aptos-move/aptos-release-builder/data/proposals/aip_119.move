script {
    use aptos_framework::aptos_governance;
    use aptos_framework::staking_config;
    use aptos_framework::block;
    use aptos_framework::fixed_point64;

    fun main(proposal_id: u64) {
        let framework_signer = &aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        let seconds_in_year: u64 = 60 * 60 * 24 * 365;

        // get the current rewards rate
        let (prev_epoch_rewards_rate_numerator, prev_epoch_rewards_rate_denominator) = staking_config::reward_rate();
        let prev_epoch_rewards_rate = fixed_point64::create_from_rational(prev_epoch_rewards_rate_numerator as u128, prev_epoch_rewards_rate_denominator as u128);

        // get the number of epochs in a year
        let epoch_seconds = block::get_epoch_interval_secs();
        let num_epochs_in_a_year = seconds_in_year / epoch_seconds;
        // AIP reduction is 25 basis points per year, we multiply the denominator by the number of epochs in a year
        // to get the reduction per epoch, that accumulates over the year as a 25bps reduction
        let aip_reduction = fixed_point64::create_from_rational(25, 10_000 * (num_epochs_in_a_year as u128));
        // subtract the AIP reduction from the previous rewards rate
        let new_rewards_rate = prev_epoch_rewards_rate.sub(aip_reduction);

        // remaining values are unchanged
        // "raw" values are stored in 0x1::staking_config::StakingRewardsConfig
        // StakingRewardsConfig can be found here:
        // https://mainnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::staking_config::StakingRewardsConfig
        let min_rewards_rate = fixed_point64::create_from_raw_value(136874841026924);
        let rewards_rate_period_sec = seconds_in_year; // unchanged, 1 year
        let rewards_rate_decrease_rate = fixed_point64::create_from_raw_value(276701161105643274); // unchanged
    
        // Update rewards config to new rewards rate
        staking_config::update_rewards_config(
            framework_signer,
            new_rewards_rate,
            min_rewards_rate,
            rewards_rate_period_sec,
            rewards_rate_decrease_rate,
        );

        // Trigger reconfiguration for changes to take effect immediately
        aptos_governance::reconfigure(framework_signer);
    }
}