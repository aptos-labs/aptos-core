/// Provides the configuration for staking and rewards
module aptos_framework::staking_config {
    use std::error;

    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    /// Stake lockup duration cannot be zero.
    const EZERO_LOCKUP_DURATION: u64 = 1;
    /// Reward rate denominator cannot be zero.
    const EZERO_REWARDS_RATE_DENOMINATOR: u64 = 2;
    /// Specified stake range is invalid. Max must be greater than min.
    const EINVALID_STAKE_RANGE: u64 = 3;
    /// The voting power increase limit percentage must be within (0, 50].
    const EINVALID_VOTING_POWER_INCREASE_LIMIT: u64 = 4;
    /// Specified rewards rate is invalid, which must be within [0, MAX_REWARDS_RATE].
    const EINVALID_REWARDS_RATE: u64 = 5;

    /// Limit the maximum value of `rewards_rate` in order to avoid any arithmetic overflow.
    const MAX_REWARDS_RATE: u64 = 1000000;

    /// Validator set configurations that will be stored with the @aptos_framework account.
    struct StakingConfig has copy, drop, key {
        // A validator needs to stake at least this amount to be able to join the validator set.
        // If after joining the validator set and at the start of any epoch, a validator's stake drops below this amount
        // they will be removed from the set.
        minimum_stake: u64,
        // A validator can only stake at most this amount. Any larger stake will be rejected.
        // If after joining the validator set and at the start of any epoch, a validator's stake exceeds this amount,
        // their voting power and rewards would only be issued for the max stake amount.
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        // Whether validators are allow to join/leave post genesis.
        allow_validator_set_change: bool,
        // The maximum rewards given out every epoch. This will be divided by the rewards rate denominator.
        // For example, 0.001% (0.00001) can be represented as 10 / 1000000.
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        // Only this % of current total voting power is allowed to join the validator set in each epoch.
        // This is necessary to prevent a massive amount of new stake from joining that can potentially take down the
        // network if corresponding validators are not ready to participate in consensus in time.
        // This value is within (0, 100%), not inclusive.
        voting_power_increase_limit: u64,
    }

    /// Only called during genesis.
    public(friend) fun initialize(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        // This can fail genesis but is necessary so that any misconfigurations can be corrected before genesis succeeds
        validate_required_stake(minimum_stake, maximum_stake);

        assert!(
            rewards_rate_denominator > 0,
            error::invalid_argument(EZERO_REWARDS_RATE_DENOMINATOR),
        );

        assert!(
            voting_power_increase_limit > 0 && voting_power_increase_limit <= 50,
            error::invalid_argument(EINVALID_VOTING_POWER_INCREASE_LIMIT),
        );

        // `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic
        // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards
        // rate (i.e., rewards_rate / rewards_rate_denominator).
        assert!(rewards_rate <= MAX_REWARDS_RATE, error::invalid_argument(EINVALID_REWARDS_RATE));

        // We assert that (rewards_rate / rewards_rate_denominator <= 1).
        assert!(rewards_rate <= rewards_rate_denominator, error::invalid_argument(EINVALID_REWARDS_RATE));

        move_to(aptos_framework, StakingConfig {
            minimum_stake,
            maximum_stake,
            recurring_lockup_duration_secs,
            allow_validator_set_change,
            rewards_rate,
            rewards_rate_denominator,
            voting_power_increase_limit,
        });
    }

    public fun get(): StakingConfig acquires StakingConfig {
        *borrow_global<StakingConfig>(@aptos_framework)
    }

    /// Return whether validator set changes are allowed
    public fun get_allow_validator_set_change(config: &StakingConfig): bool {
        config.allow_validator_set_change
    }

    /// Return the required min/max stake.
    public fun get_required_stake(config: &StakingConfig): (u64, u64) {
        (config.minimum_stake, config.maximum_stake)
    }

    /// Return the recurring lockup duration that every validator is automatically renewed for (unless they unlock and
    /// withdraw all funds).
    public fun get_recurring_lockup_duration(config: &StakingConfig): u64 {
        config.recurring_lockup_duration_secs
    }

    /// Return the reward rate.
    public fun get_reward_rate(config: &StakingConfig): (u64, u64) {
        (config.rewards_rate, config.rewards_rate_denominator)
    }

    /// Return the joining limit %.
    public fun get_voting_power_increase_limit(config: &StakingConfig): u64 {
        config.voting_power_increase_limit
    }

    /// Update the min and max stake amounts.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_required_stake(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
    ) acquires StakingConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        validate_required_stake(minimum_stake, maximum_stake);

        let staking_config = borrow_global_mut<StakingConfig>(@aptos_framework);
        staking_config.minimum_stake = minimum_stake;
        staking_config.maximum_stake = maximum_stake;
    }

    /// Update the recurring lockup duration.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_recurring_lockup_duration_secs(
        aptos_framework: &signer,
        new_recurring_lockup_duration_secs: u64,
    ) acquires StakingConfig {
        assert!(new_recurring_lockup_duration_secs > 0, error::invalid_argument(EZERO_LOCKUP_DURATION));
        system_addresses::assert_aptos_framework(aptos_framework);

        let staking_config = borrow_global_mut<StakingConfig>(@aptos_framework);
        staking_config.recurring_lockup_duration_secs = new_recurring_lockup_duration_secs;
    }

    /// Update the rewards rate.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_rewards_rate(
        aptos_framework: &signer,
        new_rewards_rate: u64,
        new_rewards_rate_denominator: u64,
    ) acquires StakingConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            new_rewards_rate_denominator > 0,
            error::invalid_argument(EZERO_REWARDS_RATE_DENOMINATOR),
        );
        // `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic
        // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards
        // rate (i.e., rewards_rate / rewards_rate_denominator).
        assert!(new_rewards_rate <= MAX_REWARDS_RATE, error::invalid_argument(EINVALID_REWARDS_RATE));

        // We assert that (rewards_rate / rewards_rate_denominator <= 1).
        assert!(new_rewards_rate <= new_rewards_rate_denominator, error::invalid_argument(EINVALID_REWARDS_RATE));

        let staking_config = borrow_global_mut<StakingConfig>(@aptos_framework);
        staking_config.rewards_rate = new_rewards_rate;
        staking_config.rewards_rate_denominator = new_rewards_rate_denominator;
    }

    /// Update the joining limit %.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_voting_power_increase_limit(
        aptos_framework: &signer,
        new_voting_power_increase_limit: u64,
    ) acquires StakingConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            new_voting_power_increase_limit > 0 && new_voting_power_increase_limit <= 50,
            error::invalid_argument(EINVALID_VOTING_POWER_INCREASE_LIMIT),
        );

        let staking_config = borrow_global_mut<StakingConfig>(@aptos_framework);
        staking_config.voting_power_increase_limit = new_voting_power_increase_limit;
    }

    fun validate_required_stake(minimum_stake: u64, maximum_stake: u64) {
        assert!(minimum_stake <= maximum_stake && maximum_stake > 0, error::invalid_argument(EINVALID_STAKE_RANGE));
    }

    #[test(aptos_framework = @aptos_framework)]
    public entry fun test_change_staking_configs(aptos_framework: signer) acquires StakingConfig {
        initialize(&aptos_framework, 0, 1, 1, false, 1, 1, 1);

        update_required_stake(&aptos_framework, 100, 1000);
        update_recurring_lockup_duration_secs(&aptos_framework, 10000);
        update_rewards_rate(&aptos_framework, 10, 100);
        update_voting_power_increase_limit(&aptos_framework, 10);

        let config = borrow_global<StakingConfig>(@aptos_framework);
        assert!(config.minimum_stake == 100, 0);
        assert!(config.maximum_stake == 1000, 1);
        assert!(config.recurring_lockup_duration_secs == 10000, 3);
        assert!(config.rewards_rate == 10, 4);
        assert!(config.rewards_rate_denominator == 100, 4);
        assert!(config.voting_power_increase_limit == 10, 5);
    }

    #[test(account = @0x123)]
    #[expected_failure(abort_code = 0x50003)]
    public entry fun test_update_required_stake_unauthorized_should_fail(account: signer) acquires StakingConfig {
        update_required_stake(&account, 1, 2);
    }

    #[test(account = @0x123)]
    #[expected_failure(abort_code = 0x50003)]
    public entry fun test_update_required_lockup_unauthorized_should_fail(account: signer) acquires StakingConfig {
        update_recurring_lockup_duration_secs(&account, 1);
    }

    #[test(account = @0x123)]
    #[expected_failure(abort_code = 0x50003)]
    public entry fun test_update_rewards_unauthorized_should_fail(account: signer) acquires StakingConfig {
        update_rewards_rate(&account, 1, 10);
    }

    #[test(account = @0x123)]
    #[expected_failure(abort_code = 0x50003)]
    public entry fun test_update_voting_power_increase_limit_unauthorized_should_fail(account: signer) acquires StakingConfig {
        update_voting_power_increase_limit(&account, 10);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003)]
    public entry fun test_update_required_stake_invalid_range_should_fail(aptos_framework: signer) acquires StakingConfig {
        update_required_stake(&aptos_framework, 10, 5);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003)]
    public entry fun test_update_required_stake_zero_max_stake_should_fail(aptos_framework: signer) acquires StakingConfig {
        update_required_stake(&aptos_framework, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10001)]
    public entry fun test_update_required_lockup_to_zero_should_fail(aptos_framework: signer) acquires StakingConfig {
        update_recurring_lockup_duration_secs(&aptos_framework, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10002)]
    public entry fun test_update_rewards_invalid_denominator_should_fail(aptos_framework: signer) acquires StakingConfig {
        update_rewards_rate(&aptos_framework, 1, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10004)]
    public entry fun test_update_voting_power_increase_limit_to_zero_should_fail(
        aptos_framework: signer
    ) acquires StakingConfig {
        update_voting_power_increase_limit(&aptos_framework, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10004)]
    public entry fun test_update_voting_power_increase_limit_to_more_than_upper_bound_should_fail(
        aptos_framework: signer
    ) acquires StakingConfig {
        update_voting_power_increase_limit(&aptos_framework, 51);
    }

    // For tests to bypass all validations.
    #[test_only]
    public fun initialize_for_test(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        if (!exists<StakingConfig>(@aptos_framework)) {
            move_to(aptos_framework, StakingConfig {
                minimum_stake,
                maximum_stake,
                recurring_lockup_duration_secs,
                allow_validator_set_change,
                rewards_rate,
                rewards_rate_denominator,
                voting_power_increase_limit,
            });
        };
    }
}
