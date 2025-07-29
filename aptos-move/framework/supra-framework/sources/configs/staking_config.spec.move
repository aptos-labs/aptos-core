spec supra_framework::staking_config {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The ability to initialize the staking config and staking rewards resources, as well as the ability to
    /// update the staking config and staking rewards should only be available to the Supra framework account.
    /// Criticality: Medium
    /// Implementation: The function initialize and initialize_rewards are used to initialize the StakingConfig and
    /// StakingRewardConfig resources. Updating the resources, can be done using the update_required_stake,
    /// update_recurring_lockup_duration_secs, update_rewards_rate, update_rewards_config,
    /// update_voting_power_increase_limit functions, which ensure that the signer is supra_framework using the
    /// assert_supra_framework function.
    /// Enforcement: Verified via [high-level-req-1.1](initialize), [high-level-req-1.2](initialize_rewards), [high-level-req-1.3](update_required_stake), [high-level-req-1.4](update_recurring_lockup_duration_secs), [high-level-req-1.5](update_rewards_rate), [high-level-req-1.6](update_rewards_config), and [high-level-req-1.7](update_voting_power_increase_limit).
    ///
    /// No.: 2
    /// Requirement: The voting power increase, in a staking config resource, should always be greater than 0 and less or
    /// equal to 50.
    /// Criticality: High
    /// Implementation: During the initialization and update of the staking config, the value of
    /// voting_power_increase_limit is ensured to be in the range of (0 to 50].
    /// Enforcement: Ensured via [high-level-req-2.1](initialize) and [high-level-req-2.2](update_voting_power_increase_limit). Formally verified via [high-level-req-2.3](StakingConfig).
    ///
    /// No.: 3
    /// Requirement: The recurring lockup duration, in a staking config resource, should always be greater than 0.
    /// Criticality: Medium
    /// Implementation: During the initialization and update of the staking config, the value of
    /// recurring_lockup_duration_secs is ensured to be greater than 0.
    /// Enforcement: Ensured via [high-level-req-3.1](initialize) and [high-level-req-3.2](update_recurring_lockup_duration_secs). Formally verified via [high-level-req-3.3](StakingConfig).
    ///
    /// No.: 4
    /// Requirement: The calculation of rewards should not be possible if the last reward rate period just started.
    /// Criticality: High
    /// Implementation: The function calculate_and_save_latest_rewards_config ensures that
    /// last_rewards_rate_period_start_in_secs is greater or equal to the current timestamp.
    /// Enforcement: Formally verified in [high-level-req-4](StakingRewardsConfigEnabledRequirement).
    ///
    /// No.: 5
    /// Requirement: The rewards rate should always be less than or equal to 100%.
    /// Criticality: High
    /// Implementation: When initializing and updating the rewards rate, it is ensured that the rewards_rate is less or
    /// equal to MAX_REWARDS_RATE, otherwise rewards rate will be negative.
    /// Enforcement: Verified via [high-level-req-5](StakingConfig).
    ///
    /// No.: 6
    /// Requirement: The reward rate's denominator should never be 0.
    /// Criticality: High
    /// Implementation: While initializing and updating the rewards rate, rewards_rate_denominator is ensured to be
    /// greater than 0.
    /// Enforcement: Verified via [high-level-req-6](StakingConfig).
    ///
    /// No.: 7
    /// Requirement: The reward rate's nominator and dominator ratio should always be less or equal to 1.
    /// Criticality: High
    /// Implementation: When initializing and updating the rewards rate, it is ensured that rewards_rate is less or
    /// equal to rewards_rate_denominator.
    /// Enforcement: Verified via [high-level-req-7](StakingConfig).
    /// </high-level-req>
    ///
    spec module {
        use supra_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<StakingConfig>(@supra_framework);
        pragma verify = true;
        // pragma aborts_if_is_strict;
    }

    spec StakingConfig {
        // `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic
        // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards
        // rate (i.e., rewards_rate / rewards_rate_denominator).
        /// [high-level-req-5]
        invariant rewards_rate <= MAX_REWARDS_RATE;
        /// [high-level-req-6]
        invariant rewards_rate_denominator > 0;
        /// [high-level-req-7]
        invariant rewards_rate <= rewards_rate_denominator;
        /// [high-level-req-3.3]
        invariant recurring_lockup_duration_secs > 0;
        /// [high-level-req-2.3]
        invariant voting_power_increase_limit > 0 && voting_power_increase_limit <= 50;
    }

    spec StakingRewardsConfig {
        invariant fixed_point64::spec_less_or_equal(
            rewards_rate,
            fixed_point64::spec_create_from_u128((1u128)));
        invariant fixed_point64::spec_less_or_equal(min_rewards_rate, rewards_rate);
        invariant rewards_rate_period_in_secs > 0;
        invariant fixed_point64::spec_ceil(rewards_rate_decrease_rate) <= 1;
    }

    /// Caller must be @supra_framework.
    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    /// The rewards_rate_denominator must greater than zero.
    /// Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
    /// The `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic overflow in the rewards calculation.
    /// rewards_rate/rewards_rate_denominator <= 1.
    /// StakingConfig does not exist under the supra_framework before creating it.
    spec initialize(
        supra_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.1]
        aborts_if addr != @supra_framework;
        aborts_if minimum_stake > maximum_stake || maximum_stake == 0;
        /// [high-level-req-3.1]
        aborts_if recurring_lockup_duration_secs == 0;
        aborts_if rewards_rate_denominator == 0;
        /// [high-level-req-2.1]
        aborts_if voting_power_increase_limit == 0 || voting_power_increase_limit > 50;
        aborts_if rewards_rate > MAX_REWARDS_RATE;
        aborts_if rewards_rate > rewards_rate_denominator;
        aborts_if exists<StakingConfig>(addr);
        ensures exists<StakingConfig>(addr);
    }

    /// Caller must be @supra_framework.
    /// last_rewards_rate_period_start_in_secs cannot be later than now.
    /// Abort at any condition in StakingRewardsConfigValidationAborts.
    /// StakingRewardsConfig does not exist under the supra_framework before creating it.
    spec initialize_rewards(
        supra_framework: &signer,
        rewards_rate: FixedPoint64,
        min_rewards_rate: FixedPoint64,
        rewards_rate_period_in_secs: u64,
        last_rewards_rate_period_start_in_secs: u64,
        rewards_rate_decrease_rate: FixedPoint64,
    ) {
        use std::signer;
        pragma verify_duration_estimate = 120;
        requires exists<timestamp::CurrentTimeMicroseconds>(@supra_framework);
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.2]
        aborts_if addr != @supra_framework;
        aborts_if last_rewards_rate_period_start_in_secs > timestamp::spec_now_seconds();
        include StakingRewardsConfigValidationAbortsIf;
        aborts_if exists<StakingRewardsConfig>(addr);
        ensures exists<StakingRewardsConfig>(addr);
    }

    spec get(): StakingConfig {
        aborts_if !exists<StakingConfig>(@supra_framework);
    }

    spec get_reward_rate(config: &StakingConfig): (u64, u64) {
        include StakingRewardsConfigRequirement;
        ensures (features::spec_periodical_reward_rate_decrease_enabled() &&
            (global<StakingRewardsConfig>(@supra_framework).rewards_rate.value as u64) != 0) ==>
                result_1 <= MAX_REWARDS_RATE && result_2 <= MAX_U64;
    }

    spec reward_rate(): (u64, u64) {
        let config = global<StakingConfig>(@supra_framework);
        aborts_if !exists<StakingConfig>(@supra_framework);
        include StakingRewardsConfigRequirement;
        ensures (features::spec_periodical_reward_rate_decrease_enabled() &&
            (global<StakingRewardsConfig>(@supra_framework).rewards_rate.value as u64) != 0) ==>
            result_1 <= MAX_REWARDS_RATE && result_2 <= MAX_U64;
    }

    spec calculate_and_save_latest_epoch_rewards_rate(): FixedPoint64 {
        pragma verify_duration_estimate = 120;
        aborts_if !exists<StakingRewardsConfig>(@supra_framework);
        aborts_if !features::spec_periodical_reward_rate_decrease_enabled();
        include StakingRewardsConfigRequirement;
    }

    spec calculate_and_save_latest_rewards_config(): StakingRewardsConfig {
        pragma verify_duration_estimate = 120;
        requires features::spec_periodical_reward_rate_decrease_enabled();
        include StakingRewardsConfigRequirement;
        aborts_if !exists<StakingRewardsConfig>(@supra_framework);
    }

    /// Caller must be @supra_framework.
    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    /// The StakingConfig is under @supra_framework.
    spec update_required_stake(
        supra_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.3]
        aborts_if addr != @supra_framework;
        aborts_if minimum_stake > maximum_stake || maximum_stake == 0;
        aborts_if !exists<StakingConfig>(@supra_framework);
        ensures global<StakingConfig>(@supra_framework).minimum_stake == minimum_stake &&
            global<StakingConfig>(@supra_framework).maximum_stake == maximum_stake;
    }

    /// Caller must be @supra_framework.
    /// The new_recurring_lockup_duration_secs must greater than zero.
    /// The StakingConfig is under @supra_framework.
    spec update_recurring_lockup_duration_secs(
        supra_framework: &signer,
        new_recurring_lockup_duration_secs: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.4]
        aborts_if addr != @supra_framework;
        /// [high-level-req-3.2]
        aborts_if new_recurring_lockup_duration_secs == 0;
        aborts_if !exists<StakingConfig>(@supra_framework);
        ensures global<StakingConfig>(@supra_framework).recurring_lockup_duration_secs == new_recurring_lockup_duration_secs;
    }

    /// Caller must be @supra_framework.
    /// The new_rewards_rate_denominator must greater than zero.
    /// The StakingConfig is under @supra_framework.
    /// The `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic overflow in the rewards calculation.
    /// rewards_rate/rewards_rate_denominator <= 1.
    spec update_rewards_rate(
        supra_framework: &signer,
        new_rewards_rate: u64,
        new_rewards_rate_denominator: u64,
    ) {
        use std::signer;
        aborts_if features::spec_periodical_reward_rate_decrease_enabled();
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.5]
        aborts_if addr != @supra_framework;
        aborts_if new_rewards_rate_denominator == 0;
        aborts_if !exists<StakingConfig>(@supra_framework);
        aborts_if new_rewards_rate > MAX_REWARDS_RATE;
        aborts_if new_rewards_rate > new_rewards_rate_denominator;
        let post staking_config = global<StakingConfig>(@supra_framework);
        ensures staking_config.rewards_rate == new_rewards_rate;
        ensures staking_config.rewards_rate_denominator == new_rewards_rate_denominator;
    }

    /// Caller must be @supra_framework.
    /// StakingRewardsConfig is under the @supra_framework.
    spec update_rewards_config(
        supra_framework: &signer,
        rewards_rate: FixedPoint64,
        min_rewards_rate: FixedPoint64,
        rewards_rate_period_in_secs: u64,
        rewards_rate_decrease_rate: FixedPoint64,
    ) {
        use std::signer;
        pragma verify_duration_estimate = 120; // verified but takes long
        include StakingRewardsConfigRequirement;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.6]
        aborts_if addr != @supra_framework;
        aborts_if global<StakingRewardsConfig>(@supra_framework).rewards_rate_period_in_secs != rewards_rate_period_in_secs;
        include StakingRewardsConfigValidationAbortsIf;
        aborts_if !exists<StakingRewardsConfig>(addr);
        let post staking_rewards_config = global<StakingRewardsConfig>(@supra_framework);
        ensures staking_rewards_config.rewards_rate == rewards_rate;
        ensures staking_rewards_config.min_rewards_rate == min_rewards_rate;
        ensures staking_rewards_config.rewards_rate_period_in_secs == rewards_rate_period_in_secs;
        ensures staking_rewards_config.rewards_rate_decrease_rate == rewards_rate_decrease_rate;
    }

    /// Caller must be @supra_framework.
    /// Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
    /// The StakingConfig is under @supra_framework.
    spec update_voting_power_increase_limit(
        supra_framework: &signer,
        new_voting_power_increase_limit: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1.7]
        aborts_if addr != @supra_framework;
        /// [high-level-req-2.2]
        aborts_if new_voting_power_increase_limit == 0 || new_voting_power_increase_limit > 50;
        aborts_if !exists<StakingConfig>(@supra_framework);
        ensures global<StakingConfig>(@supra_framework).voting_power_increase_limit == new_voting_power_increase_limit;
    }

    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    spec validate_required_stake(minimum_stake: u64, maximum_stake: u64) {
        aborts_if minimum_stake > maximum_stake || maximum_stake == 0;
    }

    /// Abort at any condition in StakingRewardsConfigValidationAborts.
    spec validate_rewards_config(
        rewards_rate: FixedPoint64,
        min_rewards_rate: FixedPoint64,
        rewards_rate_period_in_secs: u64,
        rewards_rate_decrease_rate: FixedPoint64,
    ) {
        include StakingRewardsConfigValidationAbortsIf;
    }

    /// rewards_rate must be within [0, 1].
    /// min_rewards_rate must be not greater than rewards_rate.
    /// rewards_rate_period_in_secs must be greater than 0.
    /// rewards_rate_decrease_rate must be within [0,1].
    spec schema StakingRewardsConfigValidationAbortsIf {
        rewards_rate: FixedPoint64;
        min_rewards_rate: FixedPoint64;
        rewards_rate_period_in_secs: u64;
        rewards_rate_decrease_rate: FixedPoint64;

        aborts_if fixed_point64::spec_greater(
            rewards_rate,
            fixed_point64::spec_create_from_u128((1u128)));
        aborts_if fixed_point64::spec_greater(min_rewards_rate, rewards_rate);
        aborts_if rewards_rate_period_in_secs == 0;
        aborts_if fixed_point64::spec_ceil(rewards_rate_decrease_rate) > 1;
    }

    spec schema StakingRewardsConfigRequirement {
        requires exists<timestamp::CurrentTimeMicroseconds>(@supra_framework);
        include features::spec_periodical_reward_rate_decrease_enabled() ==> StakingRewardsConfigEnabledRequirement;
    }

    spec schema StakingRewardsConfigEnabledRequirement {
        requires exists<StakingRewardsConfig>(@supra_framework);
        let staking_rewards_config = global<StakingRewardsConfig>(@supra_framework);
        let rewards_rate = staking_rewards_config.rewards_rate;
        let min_rewards_rate = staking_rewards_config.min_rewards_rate;
        let rewards_rate_period_in_secs = staking_rewards_config.rewards_rate_period_in_secs;
        let last_rewards_rate_period_start_in_secs = staking_rewards_config.last_rewards_rate_period_start_in_secs;
        let rewards_rate_decrease_rate = staking_rewards_config.rewards_rate_decrease_rate;

        requires fixed_point64::spec_less_or_equal(
            rewards_rate,
            fixed_point64::spec_create_from_u128((1u128)));
        requires fixed_point64::spec_less_or_equal(min_rewards_rate, rewards_rate);
        requires rewards_rate_period_in_secs > 0;
        /// [high-level-req-4]
        requires last_rewards_rate_period_start_in_secs <= timestamp::spec_now_seconds();
        requires fixed_point64::spec_ceil(rewards_rate_decrease_rate) <= 1;
    }
}
