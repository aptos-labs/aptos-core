spec aptos_framework::staking_config {
    spec module {
        use aptos_framework::chain_status;
        invariant chain_status::is_operating() ==> exists<StakingConfig>(@aptos_framework);
        invariant chain_status::is_operating() ==> exists<StakingRewardsConfig>(@aptos_framework);
    }
    spec StakingConfig {
        invariant minimum_stake <= maximum_stake;
        invariant maximum_stake > 0;
        invariant recurring_lockup_duration_secs > 0;
        invariant voting_power_increase_limit > 0;
        invariant voting_power_increase_limit <= 50;
    }
    spec StakingRewardsConfig {
        // `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic
        // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards
        // rate (i.e., rewards_rate / rewards_rate_denominator).
        invariant yearly_rewards_rate <= MAX_REWARDS_RATE;
        invariant rewards_rate_denominator > 0;
        invariant yearly_rewards_rate <= rewards_rate_denominator;
        invariant min_yearly_rewards_rate <= yearly_rewards_rate;
        invariant yearly_rewards_rate_decrease_numerator <= yearly_rewards_rate_decrease_denominator;
    }
}
