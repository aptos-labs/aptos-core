spec aptos_framework::staking_config {
    spec module {
        use aptos_framework::chain_status;
        invariant chain_status::is_operating() ==> exists<StakingConfig>(@aptos_framework);
        pragma verify = ture;
        pragma aborts_if_is_strict;
    }

    spec StakingConfig {
        // `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic
        // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards
        // rate (i.e., rewards_rate / rewards_rate_denominator).
        invariant rewards_rate <= MAX_REWARDS_RATE;
        invariant rewards_rate_denominator > 0;
        invariant rewards_rate <= rewards_rate_denominator;
    }

    /// Caller must be @aptos_framework.
    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    /// The rewards_rate_denominator must greater than zero.
    /// Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
    /// The `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic overflow in the rewards calculation.
    /// rewards_rate/rewards_rate_denominator <= 1.
    /// StakingConfig does not exist under the aptos_framework before creating it.
    spec initialize(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if minimum_stake > maximum_stake || maximum_stake <= 0;
        aborts_if recurring_lockup_duration_secs <= 0;
        aborts_if rewards_rate_denominator <= 0;
        aborts_if voting_power_increase_limit <= 0 || voting_power_increase_limit > 50;
        aborts_if rewards_rate > MAX_REWARDS_RATE;
        aborts_if rewards_rate > rewards_rate_denominator;
        aborts_if exists<StakingConfig>(addr);
    }

    spec get(): StakingConfig {
        aborts_if !exists<StakingConfig>(@aptos_framework);
    }

    /// Caller must be @aptos_framework.
    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    /// The StakingConfig is under @aptos_framework.
    spec update_required_stake(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if minimum_stake > maximum_stake || maximum_stake <= 0;
        aborts_if !exists<StakingConfig>(@aptos_framework);
    }

    /// Caller must be @aptos_framework.
    /// The new_recurring_lockup_duration_secs must greater than zero.
    /// The StakingConfig is under @aptos_framework.
    spec update_recurring_lockup_duration_secs(
        aptos_framework: &signer,
        new_recurring_lockup_duration_secs: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if new_recurring_lockup_duration_secs <= 0;
        aborts_if !exists<StakingConfig>(@aptos_framework);
    }

    /// Caller must be @aptos_framework.
    /// The new_rewards_rate_denominator must greater than zero.
    /// The StakingConfig is under @aptos_framework.
    /// The `rewards_rate` which is the numerator is limited to be `<= MAX_REWARDS_RATE` in order to avoid the arithmetic overflow in the rewards calculation.
    /// rewards_rate/rewards_rate_denominator <= 1.
    spec update_rewards_rate(
        aptos_framework: &signer,
        new_rewards_rate: u64,
        new_rewards_rate_denominator: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if new_rewards_rate_denominator <= 0;
        aborts_if !exists<StakingConfig>(@aptos_framework);
        aborts_if new_rewards_rate > MAX_REWARDS_RATE;
        aborts_if new_rewards_rate > new_rewards_rate_denominator;
    }

    /// Caller must be @aptos_framework.
    /// Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
    /// The StakingConfig is under @aptos_framework.
    spec update_voting_power_increase_limit(
        aptos_framework: &signer,
        new_voting_power_increase_limit: u64,
    ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if new_voting_power_increase_limit <= 0 || new_voting_power_increase_limit > 50;
        aborts_if !exists<StakingConfig>(@aptos_framework);
    }

    /// The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
    spec validate_required_stake(minimum_stake: u64, maximum_stake: u64) {
        aborts_if minimum_stake > maximum_stake || maximum_stake <= 0;
    }
}
