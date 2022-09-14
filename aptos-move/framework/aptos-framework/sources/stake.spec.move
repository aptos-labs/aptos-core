spec aptos_framework::stake {

    // -----------------
    // Global invariants
    // -----------------

    spec module {
        // The validator set should satisfy its desired invariant.
        invariant [suspendable] validator_set_is_valid();
        // After genesis, `AptosCoinCapabilities`, `ValidatorPerformance` and `ValidatorSet` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorPerformance>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorSet>(@aptos_framework);
    }

    // A desired invariant for the validator set.
    spec fun validator_set_is_valid(): bool {
        let validator_set = global<ValidatorSet>(@aptos_framework);
        spec_validators_are_initialized(validator_set.active_validators) &&
            spec_validators_are_initialized(validator_set.pending_inactive) &&
            spec_validators_are_initialized(validator_set.pending_active) &&
            spec_validator_indices_are_valid(validator_set.active_validators) &&
            spec_validator_indices_are_valid(validator_set.pending_inactive)
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec on_new_epoch {
        pragma disable_invariants_in_body;
        // The following resource requirement cannot be discharged by the global
        // invariants because this function is called during genesis.
        include ResourceRequirement;
        // This function should never abort.
        aborts_if false;
    }

    spec update_performance_statistics {
        // This function is expected to be used after genesis.
        requires chain_status::is_operating();
        // This function should never abort.
        aborts_if false;
    }

    spec update_stake_pool {
        include ResourceRequirement;
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if global<ValidatorConfig>(pool_address).validator_index >= len(validator_perf.validators);
    }

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

    spec find_validator {
        pragma opaque;
        ensures option::is_none(result) ==> (forall i in 0..len(v): v[i].addr != addr);
        ensures option::is_some(result) ==> v[option::borrow(result)].addr == addr;
    }

    spec append {
        pragma opaque, verify=false;
        aborts_if false;
        ensures len(v1) == old(len(v1) + len(v2));
        ensures len(v2) == 0;
        // The prefix of the new `v1` is the same as the old `v1`.
        ensures (forall i in 0..old(len(v1)): v1[i] == old(v1[i]));
        // The suffix of the new `v1` is the same as the reverse of the old `v2`.
        ensures (forall i in old(len(v1))..len(v1): v1[i] == old(v2[len(v2) - (i - len(v1)) - 1]));
    }

    spec remove_validators {
        pragma disable_invariants_in_body;
    }

    spec remove_validators_internal {
        requires spec_validators_are_initialized(active_validators);
        requires spec_validator_indices_are_valid(active_validators);
    }


    // ---------------------------------
    // Spec helper functions and schemas
    // ---------------------------------

    // A predicate that all given validators have been initialized.
    spec fun spec_validators_are_initialized(validators: vector<ValidatorInfo>): bool {
        forall i in 0..len(validators):
            spec_has_stake_pool(validators[i].addr) &&
                spec_has_validator_config(validators[i].addr)
    }

    // A predicate that the validator index of each given validator in-range.
    spec fun spec_validator_indices_are_valid(validators: vector<ValidatorInfo>): bool {
        forall i in 0..len(validators):
            global<ValidatorConfig>(validators[i].addr).validator_index < spec_validator_index_upper_bound()
    }

    // The upper bound of validator indices.
    spec fun spec_validator_index_upper_bound(): u64 {
        len(global<ValidatorPerformance>(@aptos_framework).validators)
    }

    spec fun spec_has_stake_pool(a: address): bool {
        exists<StakePool>(a)
    }

    spec fun spec_has_validator_config(a: address): bool {
        exists<ValidatorConfig>(a)
    }

    // An uninterpreted spec function to represent the stake reward formula.
    spec fun spec_rewards_amount(
        stake_amount: u64,
        num_successful_proposals: u64,
        num_total_proposals: u64,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
    ): u64;

    // These resources are required to successfully execute `on_new_epoch`, which cannot
    // be discharged by the global invariants because `on_new_epoch` is called in genesis.
    spec schema ResourceRequirement {
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<ValidatorPerformance>(@aptos_framework);
        requires exists<ValidatorSet>(@aptos_framework);
        requires exists<StakingConfig>(@aptos_framework);
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }
}
