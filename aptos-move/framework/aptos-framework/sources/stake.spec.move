spec aptos_framework::stake {
    spec calculate_rewards_amount {
        pragma opaque;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals == 0) ==> result == 0;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals > 0) ==>
                           result == ((stake_amount * rewards_rate * num_successful_proposals) /
                                      (rewards_rate_denominator * num_total_proposals));
        // We assume that rewards_rate < 100 and num_successful_proposals < 86400 (1 proposal per second in a day).
        // So, the multiplication in the reward formula should not overflow.
        aborts_if [abstract] false;
        // Used an uninterpreted spec function to avoid dealing with the arithmetic overflow and non-linear arithmetic.
        ensures [abstract] result == spec_rewards_amount(
            stake_amount,
            num_successful_proposals,
            num_total_proposals,
            rewards_rate,
            rewards_rate_denominator);
    }

    spec fun spec_rewards_amount(
        stake_amount: u64,
        num_successful_proposals: u64,
        num_total_proposals: u64,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
    ): u64;

    spec distribute_rewards {
        include ResourceRequirement;
        aborts_if false;
        ensures old(stake.value) > 0 ==>
                result == spec_rewards_amount(
                    old(stake.value),
                    num_successful_proposals,
                    num_total_proposals,
                    rewards_rate,
                    rewards_rate_denominator);
        ensures old(stake.value) > 0 ==>
                stake.value == old(stake.value) + spec_rewards_amount(
                    old(stake.value),
                    num_successful_proposals,
                    num_total_proposals,
                    rewards_rate,
                    rewards_rate_denominator);
        ensures old(stake.value) == 0 ==> result == 0;
        ensures old(stake.value) == 0 ==> stake.value == old(stake.value);
    }

    spec on_new_epoch {
        // The following resource requirement cannot be discharged by the global
        // invariants because this function is called during genesis.
        include ResourceRequirement;
        // This function should never abort.
        aborts_if false;
    }

    spec update_stake_pool {
        include ResourceRequirement;
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if global<ValidatorConfig>(pool_address).validator_index >= len(validator_perf.validators);
    }

    spec find_validator {
        pragma opaque;
        ensures option::is_none(result) ==> (forall i in 0..len(v): v[i].addr != addr);
        ensures option::is_some(result) ==> v[option::borrow(result)].addr == addr;
    }

    spec update_performance_statistics {
        requires chain_status::is_operating();
        // This function should never abort.
        aborts_if false;
    }

    // These resources are required to successfully execute `on_new_epoch`.
    spec schema ResourceRequirement {
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<ValidatorPerformance>(@aptos_framework);
        requires exists<ValidatorSet>(@aptos_framework);
        requires exists<StakingConfig>(@aptos_framework);
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec fun spec_has_stake_pool(a: address): bool {
        exists<StakePool>(a)
    }

    spec fun spec_has_validator_config(a: address): bool {
        exists<ValidatorConfig>(a)
    }

    spec fun spec_is_initialized_validator(a: address): bool {
        spec_has_stake_pool(a) && spec_has_validator_config(a)
    }

    spec module {
        // After genesis, `AptosCoinCapabilities`, `ValidatorPerformance` and `ValidatorSet` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorPerformance>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorSet>(@aptos_framework);
    }
}
