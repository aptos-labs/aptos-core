spec aptos_framework::stake {
    // -----------------
    // Global invariants
    // -----------------

    spec module {
        // The validator set should satisfy its desired invariant.
        invariant [suspendable] exists<ValidatorSet>(@aptos_framework) ==> validator_set_is_valid();
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

    spec initialize_validator_fees(aptos_framework: &signer) {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<ValidatorFees>(aptos_addr);
        ensures exists<ValidatorFees>(aptos_addr);
    }

    // `Validator` is initialized once.
    spec initialize(aptos_framework: &signer) {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<ValidatorSet>(aptos_addr);
        aborts_if exists<ValidatorPerformance>(aptos_addr);
    }

    spec extract_owner_cap(owner: &signer): OwnerCapability {
        let owner_address = signer::address_of(owner);
        aborts_if !exists<OwnerCapability>(owner_address);
    }

    spec deposit_owner_cap(owner: &signer, owner_cap: OwnerCapability) {
        let owner_address = signer::address_of(owner);
        aborts_if exists<OwnerCapability>(owner_address);
    }

    spec unlock_with_cap(amount: u64, owner_cap: &OwnerCapability) {
        let pool_address = owner_cap.pool_address;
        let pre_stake_pool = global<StakePool>(pool_address);
        let post stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        let min_amount = aptos_std::math64::min(amount,pre_stake_pool.active.value);

        ensures stake_pool.pending_inactive.value == pre_stake_pool.pending_inactive.value + min_amount;
    }

    // Only active validator can update locked_until_secs.
    spec increase_lockup_with_cap(owner_cap: &OwnerCapability) {
        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let pool_address = owner_cap.pool_address;
        let pre_stake_pool = global<StakePool>(pool_address);
        let post stake_pool = global<StakePool>(pool_address);
        let now_seconds = timestamp::spec_now_seconds();
        let lockup = config.recurring_lockup_duration_secs;
        modifies global<StakePool>(pool_address);

        aborts_if !exists<StakePool>(pool_address);
        aborts_if pre_stake_pool.locked_until_secs >= lockup + now_seconds;
        aborts_if lockup + now_seconds > MAX_U64;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);

        ensures stake_pool.locked_until_secs == lockup + now_seconds;
    }

    spec update_network_and_fullnode_addresses(
        operator: &signer,
        pool_address: address,
        new_network_addresses: vector<u8>,
        new_fullnode_addresses: vector<u8>,
    ) {
        let pre_stake_pool = global<StakePool>(pool_address);
        let post validator_info = global<ValidatorConfig>(pool_address);
        modifies global<ValidatorConfig>(pool_address);

        // Only the true operator address can update the network and full node addresses of the validator.
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if signer::address_of(operator) != pre_stake_pool.operator_address;

        ensures validator_info.network_addresses == new_network_addresses;
        ensures validator_info.fullnode_addresses == new_fullnode_addresses;
    }

    spec set_operator_with_cap(owner_cap: &OwnerCapability, new_operator: address) {
        let pool_address = owner_cap.pool_address;
        let post stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        ensures stake_pool.operator_address == new_operator;
    }

    spec reactivate_stake_with_cap(owner_cap: &OwnerCapability, amount: u64) {
        let pool_address = owner_cap.pool_address;
        aborts_if !stake_pool_exists(pool_address);

        let pre_stake_pool = global<StakePool>(pool_address);
        let post stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        let min_amount = aptos_std::math64::min(amount,pre_stake_pool.pending_inactive.value);

        ensures stake_pool.active.value == pre_stake_pool.active.value + min_amount;
    }

    spec rotate_consensus_key(
        operator: &signer,
        pool_address: address,
        new_consensus_pubkey: vector<u8>,
        proof_of_possession: vector<u8>,
    ) {
        let pre_stake_pool = global<StakePool>(pool_address);
        let post validator_info = global<ValidatorConfig>(pool_address);
        modifies global<ValidatorConfig>(pool_address);

        ensures validator_info.consensus_pubkey == new_consensus_pubkey;
    }

    spec set_delegated_voter_with_cap(owner_cap: &OwnerCapability, new_voter: address) {
        let pool_address = owner_cap.pool_address;
        let post stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        ensures stake_pool.delegated_voter == new_voter;
    }

    spec on_new_epoch {
        pragma disable_invariants_in_body;
        // The following resource requirement cannot be discharged by the global
        // invariants because this function is called during genesis.
        include ResourceRequirement;
        include staking_config::StakingRewardsConfigRequirement;
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
        include staking_config::StakingRewardsConfigRequirement;
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if global<ValidatorConfig>(pool_address).validator_index >= len(validator_perf.validators);
    }

    spec distribute_rewards {
        include ResourceRequirement;
        requires rewards_rate <= MAX_REWARDS_RATE;
        requires rewards_rate_denominator > 0;
        requires rewards_rate <= rewards_rate_denominator;
        requires num_successful_proposals <= num_total_proposals;
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
        requires rewards_rate <= MAX_REWARDS_RATE;
        requires rewards_rate_denominator > 0;
        requires rewards_rate <= rewards_rate_denominator;
        requires num_successful_proposals <= num_total_proposals;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals == 0) ==> result == 0;
        ensures [concrete] (rewards_rate_denominator * num_total_proposals > 0) ==> {
            let amount = ((stake_amount * rewards_rate * num_successful_proposals) /
                (rewards_rate_denominator * num_total_proposals));
            result == amount
        };
        aborts_if false;

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
        aborts_if false;
        ensures option::is_none(result) ==> (forall i in 0..len(v): v[i].addr != addr);
        ensures option::is_some(result) ==> v[option::borrow(result)].addr == addr;
        // Additional postcondition to help the quantifier instantiation.
        ensures option::is_some(result) ==> spec_contains(v, addr);
    }

    spec append {
        pragma opaque, verify = false;
        aborts_if false;
        ensures len(v1) == old(len(v1) + len(v2));
        ensures len(v2) == 0;
        // The prefix of the new `v1` is the same as the old `v1`.
        ensures (forall i in 0..old(len(v1)): v1[i] == old(v1[i]));
        // The suffix of the new `v1` is the same as the reverse of the old `v2`.
        ensures (forall i in old(len(v1))..len(v1): v1[i] == old(v2[len(v2) - (i - len(v1)) - 1]));
    }

    spec remove_validators {
        requires chain_status::is_operating();
    }

    spec is_current_epoch_validator {
        include ResourceRequirement;
        aborts_if !spec_has_stake_pool(pool_address);
        ensures result == spec_is_current_epoch_validator(pool_address);
    }

    spec get_validator_state {
        let validator_set = global<ValidatorSet>(@aptos_framework);
        ensures result == VALIDATOR_STATUS_PENDING_ACTIVE ==> spec_contains(validator_set.pending_active, pool_address);
        ensures result == VALIDATOR_STATUS_ACTIVE ==> spec_contains(validator_set.active_validators, pool_address);
        ensures result == VALIDATOR_STATUS_PENDING_INACTIVE ==> spec_contains(validator_set.pending_inactive, pool_address);
        ensures result == VALIDATOR_STATUS_INACTIVE ==> (
            !spec_contains(validator_set.pending_active, pool_address)
                && !spec_contains(validator_set.active_validators, pool_address)
                && !spec_contains(validator_set.pending_inactive, pool_address)
        );
    }

    spec add_stake_with_cap {
        include ResourceRequirement;
    }

    spec add_stake {
        include ResourceRequirement;
    }

    spec initialize_stake_owner {
        include ResourceRequirement;
    }

    spec add_transaction_fee(validator_addr: address, fee: Coin<AptosCoin>) {
        aborts_if !exists<ValidatorFees>(@aptos_framework);
    }

    spec update_voting_power_increase(increase_amount: u64) {
        let aptos = @aptos_framework;
        let pre_validator_set = global<ValidatorSet>(aptos);
        let post validator_set = global<ValidatorSet>(aptos);
        let staking_config = global<staking_config::StakingConfig>(aptos);
        let voting_power_increase_limit = staking_config.voting_power_increase_limit;

        // Correctly modified total_joining_power and the value of total_voting_power is legal.
        ensures validator_set.total_voting_power > 0 ==> validator_set.total_joining_power <= validator_set.total_voting_power * voting_power_increase_limit / 100;
        ensures validator_set.total_joining_power == pre_validator_set.total_joining_power + increase_amount;
    }

    spec assert_stake_pool_exists(pool_address: address) {
        aborts_if !stake_pool_exists(pool_address);
    }

    spec configure_allowed_validators(aptos_framework: &signer, accounts: vector<address>) {
        let aptos_framework_address = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_framework_address);
        let post allowed = global<AllowedValidators>(aptos_framework_address);
        // Make sure that the accounts of AllowedValidators are always the passed parameter.
        ensures allowed.accounts == accounts;
    }

    spec assert_owner_cap_exists(owner: address) {
        aborts_if !exists<OwnerCapability>(owner);
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

    spec fun spec_contains(validators: vector<ValidatorInfo>, addr: address): bool {
        exists i in 0..len(validators): validators[i].addr == addr
    }

    spec fun spec_is_current_epoch_validator(pool_address: address): bool {
        let validator_set = global<ValidatorSet>(@aptos_framework);
        !spec_contains(validator_set.pending_active, pool_address)
            && (spec_contains(validator_set.active_validators, pool_address)
            || spec_contains(validator_set.pending_inactive, pool_address))
    }

    // These resources are required to successfully execute `on_new_epoch`, which cannot
    // be discharged by the global invariants because `on_new_epoch` is called in genesis.
    spec schema ResourceRequirement {
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<ValidatorPerformance>(@aptos_framework);
        requires exists<ValidatorSet>(@aptos_framework);
        requires exists<StakingConfig>(@aptos_framework);
        requires exists<StakingRewardsConfig>(@aptos_framework) || !features::spec_periodical_reward_rate_decrease_enabled();
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        requires exists<ValidatorFees>(@aptos_framework);
    }
}
