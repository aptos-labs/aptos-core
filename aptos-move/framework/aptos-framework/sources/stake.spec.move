spec aptos_framework::stake {
    spec calculate_rewards_amount {
        pragma opaque;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals == 0) ==> result == 0;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals > 0) ==>
                           result == ((stake_amount * rewards_rate * num_successful_proposals) /
                                      (rewards_rate_denominator * num_total_proposals));
        // Used an uninterpreted spec function to avoid dealing with the arithmetic overflow and non-linear arithmetic.
        aborts_if [abstract] false;
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
        requires timestamp::is_operating();
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
        aborts_if false;
    }

    spec update_stake_pool {
        requires timestamp::is_operating();
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
        aborts_if false;
    }

    spec module {
        invariant timestamp::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
        invariant timestamp::is_operating() ==> exists<ValidatorPerformance>(@aptos_framework);
        invariant timestamp::is_operating() ==> exists<ValidatorSet>(@aptos_framework);
    }
}
