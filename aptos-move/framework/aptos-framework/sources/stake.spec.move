spec aptos_framework::stake {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The validator set resource stores consensus information for each validator. The consensus scheme
    /// remains consistent across all validators within the set.
    /// Criticality: Low
    /// Implementation: The consensus_scheme attribute within ValidatorSet initializes with the value zero during the
    /// module's initialization and its value remains unchanged afterward.
    /// Enforcement: Formally verified by the data invariant of [high-level-req-1](ValidatorSet).
    ///
    /// No.: 2
    /// Requirement: The owner of a validator is immutable.
    /// Criticality: Low
    /// Implementation: During the initialization of a validator, the owner attribute becomes the signer's address. This
    /// assignment establishes the signer as the owner and controller of the validator entity. Subsequently, the owner
    /// attribute remains unchanged throughout the validator's lifespan, maintaining its assigned value without any
    /// modifications.
    /// Enforcement: Formally verified in the schema [high-level-req-2](ValidatorOwnerNoChange).
    ///
    /// No.: 3
    /// Requirement: The total staked value in the stake pool should remain constant, excluding operations related to
    /// adding and withdrawing.
    /// Criticality: Low
    /// Implementation: The total staked value (AptosCoin) of a stake pool is grouped by: active, inactive,
    /// pending_active, and pending_inactive. The stake value remains constant except during the execution of the
    /// add_stake_with_cap or withdraw_with_cap functions or on_new_epoch (which distributes the reward).
    /// Enforcement: Formally specified in the schema [high-level-req-3](StakedValueNoChange).
    ///
    /// No.: 4
    /// Requirement: During each epoch, the following operations should be consistently performed without aborting: rewards
    /// distribution, validator activation/deactivation, updates to validator sets and voting power, and renewal of
    /// lockups.
    /// Criticality: Low
    /// Implementation: The on_new_epoch function is triggered at each epoch boundary to perform distribution of the
    /// transaction fee, updates to active/inactive stakes, updates to pending active/inactive validators and adjusts
    /// voting power of the validators without aborting.
    /// Enforcement: Formally verified via [high-level-req-4](on_new_epoch). This also requires a manual review to verify the state updates of the stake pool.
    /// </high-level-req>
    ///
    // -----------------
    // Global invariants
    // -----------------
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
        // The validator set should satisfy its desired invariant.
        invariant [suspendable] exists<ValidatorSet>(@aptos_framework) ==> validator_set_is_valid();
        // After genesis, `AptosCoinCapabilities`, `ValidatorPerformance` and `ValidatorSet` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorPerformance>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<ValidatorSet>(@aptos_framework);

        // property 2: The owner of a validator remains immutable.
        apply ValidatorOwnerNoChange to *;

        apply ValidatorNotChangeDuringReconfig to * except on_new_epoch;
        apply StakePoolNotChangeDuringReconfig to * except on_new_epoch, update_stake_pool;

        // ghost variable
        global ghost_valid_perf: ValidatorPerformance;
        global ghost_proposer_idx: Option<u64>;
        global ghost_active_num: u64;
        global ghost_pending_inactive_num: u64;
    }

    // property 1: the validator set resource stores consensus information for each validator.
    // the consensus scheme remains consistent across all validators within the set.
    spec ValidatorSet {
        /// [high-level-req-1]
        invariant consensus_scheme == 0;
    }

    spec schema ValidatorNotChangeDuringReconfig {
        ensures (reconfiguration_state::spec_is_in_progress() && old(exists<ValidatorSet>(@aptos_framework))) ==>
            old(global<ValidatorSet>(@aptos_framework)) == global<ValidatorSet>(@aptos_framework);
    }

    spec schema StakePoolNotChangeDuringReconfig {
        ensures forall a: address where old(exists<StakePool>(a)): reconfiguration_state::spec_is_in_progress() ==>
            (old(global<StakePool>(a).pending_inactive) == global<StakePool>(a).pending_inactive &&
            old(global<StakePool>(a).pending_active) == global<StakePool>(a).pending_active &&
            old(global<StakePool>(a).inactive) == global<StakePool>(a).inactive &&
            old(global<StakePool>(a).active) == global<StakePool>(a).active);
    }

    spec schema ValidatorOwnerNoChange {
        /// [high-level-req-2]
        ensures forall addr: address where old(exists<OwnerCapability>(addr)):
            old(global<OwnerCapability>(addr)).pool_address == global<OwnerCapability>(addr).pool_address;
    }

    // property 3: The total staked value in the stake pool should be constant (excluding adding and withdrawing operations).
    spec schema StakedValueNochange {
        pool_address: address;
        let stake_pool = global<StakePool>(pool_address);
        let post post_stake_pool = global<StakePool>(pool_address);
        /// [high-level-req-3]
        ensures stake_pool.active.value + stake_pool.inactive.value + stake_pool.pending_active.value + stake_pool.pending_inactive.value ==
            post_stake_pool.active.value + post_stake_pool.inactive.value + post_stake_pool.pending_active.value + post_stake_pool.pending_inactive.value;
    }

    // A desired invariant for the validator set.
    spec fun validator_set_is_valid(): bool {
        let validator_set = global<ValidatorSet>(@aptos_framework);
        validator_set_is_valid_impl(validator_set)
    }

    spec fun validator_set_is_valid_impl(validator_set: ValidatorSet): bool {
        spec_validators_are_initialized(validator_set.active_validators) &&
            spec_validators_are_initialized(validator_set.pending_inactive) &&
            spec_validators_are_initialized(validator_set.pending_active) &&
            spec_validator_indices_are_valid(validator_set.active_validators) &&
            spec_validator_indices_are_valid(validator_set.pending_inactive)
            && spec_validator_indices_active_pending_inactive(validator_set)
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec initialize_validator(
        account: &signer,
        consensus_pubkey: vector<u8>,
        proof_of_possession: vector<u8>,
        network_addresses: vector<u8>,
        fullnode_addresses: vector<u8>,
    ){
        pragma verify = false;

        include AbortsIfSignerPermissionStake {
            s: account
        };
        let pubkey_from_pop = bls12381::spec_public_key_from_bytes_with_pop(
            consensus_pubkey,
            proof_of_possession_from_bytes(proof_of_possession)
        );
        aborts_if !option::spec_is_some(pubkey_from_pop);
        let addr = signer::address_of(account);
        let post_addr = signer::address_of(account);
        let allowed = global<AllowedValidators>(@aptos_framework);
        aborts_if exists<ValidatorConfig>(addr);
        aborts_if exists<AllowedValidators>(@aptos_framework) && !vector::spec_contains(allowed.accounts, addr);
        aborts_if stake_pool_exists(addr);
        aborts_if exists<OwnerCapability>(addr);
        aborts_if !exists<account::Account>(addr);
        aborts_if global<account::Account>(addr).guid_creation_num + 12 > MAX_U64;
        aborts_if global<account::Account>(addr).guid_creation_num + 12 >= account::MAX_GUID_CREATION_NUM;
        ensures exists<StakePool>(post_addr);
        ensures global<OwnerCapability>(post_addr) == OwnerCapability { pool_address: post_addr };
        ensures global<ValidatorConfig>(post_addr) == ValidatorConfig {
            consensus_pubkey,
            network_addresses,
            fullnode_addresses,
            validator_index: 0,
        };
    }

    // `Validator` is initialized once.
    spec initialize(aptos_framework: &signer) {
        pragma disable_invariants_in_body;
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<ValidatorSet>(aptos_addr);
        aborts_if exists<ValidatorPerformance>(aptos_addr);
        ensures exists<ValidatorSet>(aptos_addr);
        ensures global<ValidatorSet>(aptos_addr).consensus_scheme == 0;
        ensures exists<ValidatorPerformance>(aptos_addr);
    }

    spec join_validator_set(
        operator: &signer,
        pool_address: address
    )
    {
        // This function casue timeout (property proved)
        pragma verify_duration_estimate = 60;
        pragma disable_invariants_in_body;
        include AbortsIfSignerPermissionStake {
            s: operator
        };
        aborts_if !staking_config::get_allow_validator_set_change(staking_config::get());
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if !exists<StakingConfig>(@aptos_framework);
        aborts_if !exists<ValidatorSet>(@aptos_framework);
        aborts_if reconfiguration_state::spec_is_in_progress();

        let stake_pool = global<StakePool>(pool_address);
        let validator_set = global<ValidatorSet>(@aptos_framework);
        let post p_validator_set = global<ValidatorSet>(@aptos_framework);
        aborts_if signer::address_of(operator) != stake_pool.operator_address;
        aborts_if option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) ||
                    option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) ||
                        option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));

        let config = staking_config::get();
        let voting_power = get_next_epoch_voting_power(stake_pool);

        let minimum_stake = config.minimum_stake;
        let maximum_stake = config.maximum_stake;
        aborts_if voting_power < minimum_stake;
        aborts_if voting_power >maximum_stake;

        let validator_config = global<ValidatorConfig>(pool_address);
        aborts_if vector::is_empty(validator_config.consensus_pubkey);

        let validator_set_size = vector::length(validator_set.active_validators) + vector::length(validator_set.pending_active) + 1;
        aborts_if validator_set_size > MAX_VALIDATOR_SET_SIZE;

        let voting_power_increase_limit = (staking_config::get_voting_power_increase_limit(config) as u128);

        aborts_if (validator_set.total_joining_power + (voting_power as u128)) > MAX_U128;
        aborts_if validator_set.total_voting_power * voting_power_increase_limit > MAX_U128;
        aborts_if validator_set.total_voting_power > 0 &&
            (validator_set.total_joining_power + (voting_power as u128)) * 100 > validator_set.total_voting_power * voting_power_increase_limit;

        let post p_validator_info = ValidatorInfo {
            addr: pool_address,
            voting_power,
            config: validator_config,
        };

        ensures validator_set.total_joining_power + voting_power == p_validator_set.total_joining_power;
        ensures vector::spec_contains(p_validator_set.pending_active, p_validator_info);
    }

    spec withdraw(
        owner: &signer,
        withdraw_amount: u64
    )
    {
        // TODO(fa_migration)
        pragma verify = false;
        include AbortsIfSignerPermissionStake {
            s: owner
        };
        aborts_if reconfiguration_state::spec_is_in_progress();
        let addr = signer::address_of(owner);
        let ownership_cap = global<OwnerCapability>(addr);
        let pool_address = ownership_cap.pool_address;
        let stake_pool = global<StakePool>(pool_address);
        aborts_if !exists<OwnerCapability>(addr);
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorSet>(@aptos_framework);

        let validator_set = global<ValidatorSet>(@aptos_framework);
        let bool_find_validator = !option::spec_is_some(spec_find_validator(validator_set.active_validators, pool_address)) &&
                    !option::spec_is_some(spec_find_validator(validator_set.pending_inactive, pool_address)) &&
                        !option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));
        aborts_if bool_find_validator && !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        let new_withdraw_amount_1 = min(withdraw_amount, stake_pool.inactive.value + stake_pool.pending_inactive.value);
        let new_withdraw_amount_2 = min(withdraw_amount, stake_pool.inactive.value);
        aborts_if bool_find_validator && timestamp::now_seconds() > stake_pool.locked_until_secs &&
                    new_withdraw_amount_1 > 0 && stake_pool.inactive.value + stake_pool.pending_inactive.value < new_withdraw_amount_1;
        aborts_if !(bool_find_validator && exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework)) &&
                    new_withdraw_amount_2 > 0 && stake_pool.inactive.value < new_withdraw_amount_2;
        aborts_if !exists<coin::CoinStore<AptosCoin>>(addr);
        include coin::DepositAbortsIf<AptosCoin>{account_addr: addr};

        let coin_store = global<coin::CoinStore<AptosCoin>>(addr);
        let post p_coin_store = global<coin::CoinStore<AptosCoin>>(addr);
        ensures bool_find_validator && timestamp::now_seconds() > stake_pool.locked_until_secs
                    && exists<account::Account>(addr) && exists<coin::CoinStore<AptosCoin>>(addr) ==>
                        coin_store.coin.value + new_withdraw_amount_1 == p_coin_store.coin.value;
        ensures !(bool_find_validator && exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework))
                    && exists<account::Account>(addr) && exists<coin::CoinStore<AptosCoin>>(addr) ==>
                        coin_store.coin.value + new_withdraw_amount_2 == p_coin_store.coin.value;
    }

    spec leave_validator_set(
        operator: &signer,
        pool_address: address
    ) {
        pragma disable_invariants_in_body;
        requires chain_status::is_operating();
        include AbortsIfSignerPermissionStake {
            s: operator
        };
        aborts_if reconfiguration_state::spec_is_in_progress();
        let config = staking_config::get();
        aborts_if !staking_config::get_allow_validator_set_change(config);
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorSet>(@aptos_framework);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);
        let stake_pool = global<StakePool>(pool_address);
        aborts_if signer::address_of(operator) != stake_pool.operator_address;

        let validator_set = global<ValidatorSet>(@aptos_framework);
        let validator_find_bool = option::spec_is_some(spec_find_validator(validator_set.pending_active, pool_address));
        let active_validators = validator_set.active_validators;
        let pending_active = validator_set.pending_active;

        let post post_validator_set = global<ValidatorSet>(@aptos_framework);
        let post post_active_validators = post_validator_set.active_validators;
        let pending_inactive_validators = validator_set.pending_inactive;
        let post post_pending_inactive_validators = post_validator_set.pending_inactive;
        ensures len(active_validators) + len(pending_inactive_validators) == len(post_active_validators)
            + len(post_pending_inactive_validators);

        aborts_if !validator_find_bool && !option::spec_is_some(spec_find_validator(active_validators, pool_address));
        aborts_if !validator_find_bool && vector::length(validator_set.active_validators) <= option::spec_borrow(spec_find_validator(active_validators, pool_address));
        aborts_if !validator_find_bool && vector::length(validator_set.active_validators) < 2;
        aborts_if validator_find_bool && vector::length(validator_set.pending_active) <= option::spec_borrow(spec_find_validator(pending_active, pool_address));
        let post p_validator_set = global<ValidatorSet>(@aptos_framework);
        let validator_stake = (get_next_epoch_voting_power(stake_pool) as u128);
        ensures validator_find_bool && validator_set.total_joining_power > validator_stake ==>
                    p_validator_set.total_joining_power == validator_set.total_joining_power - validator_stake;
        ensures !validator_find_bool ==> !option::spec_is_some(spec_find_validator(p_validator_set.pending_active, pool_address));
    }

    spec extract_owner_cap(owner: &signer): OwnerCapability {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        include AbortsIfSignerPermissionStake {
            s: owner
        };
        let owner_address = signer::address_of(owner);
        aborts_if !exists<OwnerCapability>(owner_address);
        ensures !exists<OwnerCapability>(owner_address);
    }

    spec deposit_owner_cap(owner: &signer, owner_cap: OwnerCapability) {
        include AbortsIfSignerPermissionStake {
            s: owner
        };
        let owner_address = signer::address_of(owner);
        aborts_if exists<OwnerCapability>(owner_address);
        ensures exists<OwnerCapability>(owner_address);
        ensures global<OwnerCapability>(owner_address) == owner_cap;
    }

    spec unlock_with_cap(amount: u64, owner_cap: &OwnerCapability) {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        let pool_address = owner_cap.pool_address;
        let pre_stake_pool = global<StakePool>(pool_address);
        let post stake_pool = global<StakePool>(pool_address);
        aborts_if reconfiguration_state::spec_is_in_progress();
        aborts_if amount != 0 && !exists<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        include StakedValueNochange;
        let min_amount = aptos_std::math64::min(amount,pre_stake_pool.active.value);

        ensures stake_pool.active.value == pre_stake_pool.active.value - min_amount;
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
        include StakedValueNochange;

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
        include AbortsIfSignerPermissionStake {
            s: operator
        };
        let pre_stake_pool = global<StakePool>(pool_address);
        let post validator_info = global<ValidatorConfig>(pool_address);
        modifies global<ValidatorConfig>(pool_address);
        include StakedValueNochange;

        aborts_if reconfiguration_state::spec_is_in_progress();
        // Only the true operator address can update the network and full node addresses of the validator.
        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if signer::address_of(operator) != pre_stake_pool.operator_address;

        ensures validator_info.network_addresses == new_network_addresses;
        ensures validator_info.fullnode_addresses == new_fullnode_addresses;
    }

    spec set_operator_with_cap(owner_cap: &OwnerCapability, new_operator: address) {
        let pool_address = owner_cap.pool_address;
        let post post_stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        include StakedValueNochange;

        ensures post_stake_pool.operator_address == new_operator;
    }

    spec reactivate_stake_with_cap(owner_cap: &OwnerCapability, amount: u64) {
        let pool_address = owner_cap.pool_address;
        include StakedValueNochange;

        aborts_if reconfiguration_state::spec_is_in_progress();
        aborts_if !stake_pool_exists(pool_address);

        let pre_stake_pool = global<StakePool>(pool_address);
        let post stake_pool = global<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        let min_amount = aptos_std::math64::min(amount, pre_stake_pool.pending_inactive.value);

        ensures stake_pool.pending_inactive.value == pre_stake_pool.pending_inactive.value - min_amount;
        ensures stake_pool.active.value == pre_stake_pool.active.value + min_amount;
    }

    spec rotate_consensus_key(
        operator: &signer,
        pool_address: address,
        new_consensus_pubkey: vector<u8>,
        proof_of_possession: vector<u8>,
    ) {
        include AbortsIfSignerPermissionStake {
            s: operator
        };
        let pre_stake_pool = global<StakePool>(pool_address);
        let post validator_info = global<ValidatorConfig>(pool_address);
        aborts_if reconfiguration_state::spec_is_in_progress();
        aborts_if !exists<StakePool>(pool_address);
        aborts_if signer::address_of(operator) != pre_stake_pool.operator_address;
        aborts_if !exists<ValidatorConfig>(pool_address);
        let pubkey_from_pop = bls12381::spec_public_key_from_bytes_with_pop(
            new_consensus_pubkey,
            proof_of_possession_from_bytes(proof_of_possession)
        );
        aborts_if !option::spec_is_some(pubkey_from_pop);
        modifies global<ValidatorConfig>(pool_address);
        include StakedValueNochange;

        ensures validator_info.consensus_pubkey == new_consensus_pubkey;
    }

    spec set_delegated_voter_with_cap(owner_cap: &OwnerCapability, new_voter: address) {
        let pool_address = owner_cap.pool_address;
        let post post_stake_pool = global<StakePool>(pool_address);
        include StakedValueNochange;
        aborts_if !exists<StakePool>(pool_address);
        modifies global<StakePool>(pool_address);
        ensures post_stake_pool.delegated_voter == new_voter;
    }

    spec on_new_epoch {
        pragma verify = false; // TODO: set because of timeout (property proved).
        pragma disable_invariants_in_body;
        // The following resource requirement cannot be discharged by the global
        // invariants because this function is called during genesis.
        include ResourceRequirement;
        include GetReconfigStartTimeRequirement;
        include staking_config::StakingRewardsConfigRequirement;
        include aptos_framework::aptos_coin::ExistsAptosCoin;
        // This function should never abort.
        /// [high-level-req-4]
        aborts_if false;
    }

    spec update_performance_statistics {
        // This function is expected to be used after genesis.
        requires chain_status::is_operating();
        // This function should never abort.
        aborts_if false;

        let validator_perf = global<ValidatorPerformance>(@aptos_framework);
        let post post_validator_perf = global<ValidatorPerformance>(@aptos_framework);
        let validator_len = len(validator_perf.validators);
        ensures (option::spec_is_some(ghost_proposer_idx) && option::spec_borrow(ghost_proposer_idx) < validator_len) ==>
            (post_validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals ==
                validator_perf.validators[option::spec_borrow(ghost_proposer_idx)].successful_proposals + 1);
    }

    spec next_validator_consensus_infos {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        aborts_if false;
        include ResourceRequirement;
        include GetReconfigStartTimeRequirement;
        include features::spec_periodical_reward_rate_decrease_enabled() ==> staking_config::StakingRewardsConfigEnabledRequirement;
    }

    spec update_stake_pool {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        include ResourceRequirement;
        include GetReconfigStartTimeRequirement;
        include staking_config::StakingRewardsConfigRequirement;

        include UpdateStakePoolAbortsIf;

        let stake_pool = global<StakePool>(pool_address);
        let validator_config = global<ValidatorConfig>(pool_address);
        let cur_validator_perf = validator_perf.validators[validator_config.validator_index];
        let num_successful_proposals = cur_validator_perf.successful_proposals;
        let num_total_proposals = cur_validator_perf.successful_proposals + cur_validator_perf.failed_proposals;
        let rewards_rate = spec_get_reward_rate_1(staking_config);
        let rewards_rate_denominator = spec_get_reward_rate_2(staking_config);
        let rewards_amount_1 = if (stake_pool.active.value > 0) {
            spec_rewards_amount(stake_pool.active.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
        } else {
            0
        };
        let rewards_amount_2 = if (stake_pool.pending_inactive.value > 0) {
            spec_rewards_amount(stake_pool.pending_inactive.value, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
        } else {
            0
        };
        let post post_stake_pool = global<StakePool>(pool_address);
        let post post_active_value = post_stake_pool.active.value;
        let post post_pending_inactive_value = post_stake_pool.pending_inactive.value;
        let post post_inactive_value = post_stake_pool.inactive.value;
        ensures post_stake_pool.pending_active.value == 0;
        // the amount stored in the stake pool should not changed after the update
        ensures post_active_value == stake_pool.active.value + rewards_amount_1 + stake_pool.pending_active.value;
        // when current lockup cycle has expired, pending inactive should be fully unlocked and moved into inactive
        ensures if (spec_get_reconfig_start_time_secs() >= stake_pool.locked_until_secs) {
            post_pending_inactive_value == 0 &&
            post_inactive_value == stake_pool.inactive.value + stake_pool.pending_inactive.value + rewards_amount_2
        } else {
            post_pending_inactive_value == stake_pool.pending_inactive.value + rewards_amount_2
        };
    }

    spec schema AbortsIfSignerPermissionStake {
        use aptos_framework::permissioned_signer;
        s: signer;
        let perm = StakeManagementPermission {};
        aborts_if !permissioned_signer::spec_check_permission_exists(s, perm);
    }

    spec schema UpdateStakePoolAbortsIf {
        use aptos_std::type_info;

        pool_address: address;
        validator_perf: ValidatorPerformance;

        aborts_if !exists<StakePool>(pool_address);
        aborts_if !exists<ValidatorConfig>(pool_address);
        aborts_if global<ValidatorConfig>(pool_address).validator_index >= len(validator_perf.validators);

        let aptos_addr = type_info::type_of<AptosCoin>().account_address;

        let stake_pool = global<StakePool>(pool_address);

        include DistributeRewardsAbortsIf {stake: stake_pool.active};
        include DistributeRewardsAbortsIf {stake: stake_pool.pending_inactive};
    }

    spec distribute_rewards {
        pragma aborts_if_is_partial;
        include ResourceRequirement;
        requires rewards_rate <= MAX_REWARDS_RATE;
        requires rewards_rate_denominator > 0;
        requires rewards_rate <= rewards_rate_denominator;
        requires num_successful_proposals <= num_total_proposals;

        include DistributeRewardsAbortsIf;

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

    spec schema DistributeRewardsAbortsIf {
        use aptos_std::type_info;

        stake: Coin<AptosCoin>;
        num_successful_proposals: num;
        num_total_proposals: num;
        rewards_rate: num;
        rewards_rate_denominator: num;

        let stake_amount = coin::value(stake);
        let rewards_amount = if (stake_amount > 0) {
            spec_rewards_amount(stake_amount, num_successful_proposals, num_total_proposals, rewards_rate, rewards_rate_denominator)
        } else {
            0
        };
        let amount = rewards_amount;
        let addr = type_info::type_of<AptosCoin>().account_address;
        aborts_if (rewards_amount > 0) && !exists<coin::CoinInfo<AptosCoin>>(addr);
        modifies global<coin::CoinInfo<AptosCoin>>(addr);
        include (rewards_amount > 0) ==> coin::CoinAddAbortsIf<AptosCoin> { amount: amount };
    }

    spec get_reconfig_start_time_secs(): u64 {
        include GetReconfigStartTimeRequirement;
    }

    spec schema GetReconfigStartTimeRequirement {
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        include reconfiguration_state::StartTimeSecsRequirement;
    }

    spec fun spec_get_reconfig_start_time_secs(): u64 {
        if (exists<reconfiguration_state::State>(@aptos_framework)) {
            reconfiguration_state::spec_start_time_secs()
        } else {
            timestamp::spec_now_seconds()
        }
    }

    spec fun spec_get_lockup_secs(pool_address: address): u64 {
        global<StakePool>(pool_address).locked_until_secs
    }

    spec calculate_rewards_amount {
        pragma opaque;
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        pragma verify = false;
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
        ensures [abstract] result == spec_find_validator(v,addr);
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
        let validator_set = global<ValidatorSet>(@aptos_framework);
        let post post_validator_set = global<ValidatorSet>(@aptos_framework);
        let active_validators = validator_set.active_validators;
        let post post_active_validators = post_validator_set.active_validators;
        let pending_inactive_validators = validator_set.pending_inactive;
        let post post_pending_inactive_validators = post_validator_set.pending_inactive;

        invariant len(active_validators) > 0;
        ensures len(active_validators) + len(pending_inactive_validators) == len(post_active_validators)
            + len(post_pending_inactive_validators);
    }

    spec is_current_epoch_validator {
        include ResourceRequirement;
        aborts_if !spec_has_stake_pool(pool_address);
        ensures result == spec_is_current_epoch_validator(pool_address);
    }

    spec get_validator_state {
        aborts_if !exists<ValidatorSet>(@aptos_framework);
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
        pragma disable_invariants_in_body;
        pragma verify = false;
        include ResourceRequirement;
        let amount = coins.value;
        aborts_if reconfiguration_state::spec_is_in_progress();
        include AddStakeWithCapAbortsIfAndEnsures { amount };
    }

    spec add_stake {
        // TODO: fix
        pragma verify = false;
        // TODO(fa_migration)
        pragma aborts_if_is_partial;
        include AbortsIfSignerPermissionStake {
            s: owner
        };
        aborts_if reconfiguration_state::spec_is_in_progress();
        include ResourceRequirement;
        include AddStakeAbortsIfAndEnsures;
    }

    spec initialize_stake_owner(
        owner: &signer,
        initial_stake_amount: u64,
        operator: address,
        voter: address,
    ) {
        // TODO: These function failed in github CI
        pragma verify_duration_estimate = 120;
        pragma verify = false;
        pragma aborts_if_is_partial;
        include AbortsIfSignerPermissionStake {
            s: owner
        };
        include ResourceRequirement;
        let addr = signer::address_of(owner);
        ensures global<ValidatorConfig>(addr) == ValidatorConfig {
            consensus_pubkey: vector::empty(),
            network_addresses: vector::empty(),
            fullnode_addresses: vector::empty(),
            validator_index: 0,
        };
        ensures global<OwnerCapability>(addr) == OwnerCapability { pool_address: addr };
        let post stakepool = global<StakePool>(addr);
        let post active = stakepool.active.value;
        let post pending_active = stakepool.pending_active.value;
        ensures spec_is_current_epoch_validator(addr) ==>
            pending_active == initial_stake_amount;
        ensures !spec_is_current_epoch_validator(addr) ==>
            active == initial_stake_amount;
    }

    spec update_voting_power_increase(increase_amount: u64) {
        requires !reconfiguration_state::spec_is_in_progress();
        aborts_if !exists<ValidatorSet>(@aptos_framework);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);

        let aptos = @aptos_framework;
        let pre_validator_set = global<ValidatorSet>(aptos);
        let post validator_set = global<ValidatorSet>(aptos);
        let staking_config = global<staking_config::StakingConfig>(aptos);
        let voting_power_increase_limit = staking_config.voting_power_increase_limit;
        aborts_if pre_validator_set.total_joining_power + increase_amount > MAX_U128;
        aborts_if pre_validator_set.total_voting_power > 0 && pre_validator_set.total_voting_power * voting_power_increase_limit > MAX_U128;
        aborts_if pre_validator_set.total_voting_power > 0 &&
            pre_validator_set.total_joining_power + increase_amount > pre_validator_set.total_voting_power * voting_power_increase_limit / 100;
        // Correctly modified total_joining_power and the value of total_voting_power is legal.
        ensures validator_set.total_voting_power > 0 ==>
            validator_set.total_joining_power <= validator_set.total_voting_power * voting_power_increase_limit / 100;
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

    spec validator_consensus_infos_from_validator_set(validator_set: &ValidatorSet): vector<ValidatorConsensusInfo> {
        aborts_if false;
        invariant spec_validator_indices_are_valid_config(validator_set.active_validators,
            len(validator_set.active_validators) + len(validator_set.pending_inactive));
        invariant len(validator_set.pending_inactive) == 0 ||
            spec_validator_indices_are_valid_config(validator_set.pending_inactive,
                len(validator_set.active_validators) + len(validator_set.pending_inactive));
    }

    // ---------------------------------
    // Spec helper functions and schemas
    // ---------------------------------

    spec schema AddStakeWithCapAbortsIfAndEnsures {
        owner_cap: OwnerCapability;
        amount: u64;

        let pool_address = owner_cap.pool_address;
        aborts_if !exists<StakePool>(pool_address);

        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let validator_set = global<ValidatorSet>(@aptos_framework);
        let voting_power_increase_limit = config.voting_power_increase_limit;
        let post post_validator_set = global<ValidatorSet>(@aptos_framework);
        let update_voting_power_increase = amount != 0 && (spec_contains(validator_set.active_validators, pool_address)
                                                           || spec_contains(validator_set.pending_active, pool_address));
        aborts_if update_voting_power_increase && validator_set.total_joining_power + amount > MAX_U128;
        ensures update_voting_power_increase ==> post_validator_set.total_joining_power == validator_set.total_joining_power + amount;
        aborts_if update_voting_power_increase && validator_set.total_voting_power > 0
                && validator_set.total_voting_power * voting_power_increase_limit > MAX_U128;
        aborts_if update_voting_power_increase && validator_set.total_voting_power > 0
                && validator_set.total_joining_power + amount > validator_set.total_voting_power * voting_power_increase_limit / 100;
        let stake_pool = global<StakePool>(pool_address);
        let post post_stake_pool = global<StakePool>(pool_address);
        let value_pending_active = stake_pool.pending_active.value;
        let value_active = stake_pool.active.value;
        ensures amount != 0 && spec_is_current_epoch_validator(pool_address) ==> post_stake_pool.pending_active.value == value_pending_active + amount;
        ensures amount != 0 && !spec_is_current_epoch_validator(pool_address) ==> post_stake_pool.active.value == value_active + amount;
        let maximum_stake = config.maximum_stake;
        let value_pending_inactive = stake_pool.pending_inactive.value;
        let next_epoch_voting_power = value_pending_active + value_active + value_pending_inactive;
        let voting_power = next_epoch_voting_power + amount;
        aborts_if amount != 0 && voting_power > MAX_U64;
        aborts_if amount != 0 && voting_power > maximum_stake;
    }

    spec schema AddStakeAbortsIfAndEnsures {
        owner: signer;
        amount: u64;

        let owner_address = signer::address_of(owner);
        aborts_if !exists<OwnerCapability>(owner_address);

        let owner_cap = global<OwnerCapability>(owner_address);
        include AddStakeWithCapAbortsIfAndEnsures { owner_cap };
    }

    spec fun spec_is_allowed(account: address): bool {
        if (!exists<AllowedValidators>(@aptos_framework)) {
            true
        } else {
            let allowed = global<AllowedValidators>(@aptos_framework);
            contains(allowed.accounts, account)
        }
    }

    spec fun spec_find_validator(v: vector<ValidatorInfo>, addr: address): Option<u64>;

    // A predicate that all given validators have been initialized.
    spec fun spec_validators_are_initialized(validators: vector<ValidatorInfo>): bool {
        forall i in 0..len(validators):
            spec_has_stake_pool(validators[i].addr) &&
                spec_has_validator_config(validators[i].addr)
    }

    spec fun spec_validators_are_initialized_addrs(addrs: vector<address>): bool {
        forall i in 0..len(addrs):
            spec_has_stake_pool(addrs[i]) &&
                spec_has_validator_config(addrs[i])
    }


    // A predicate that the validator index of each given validator in-range.
    spec fun spec_validator_indices_are_valid(validators: vector<ValidatorInfo>): bool {
        spec_validator_indices_are_valid_addr(validators, spec_validator_index_upper_bound()) &&
            spec_validator_indices_are_valid_config(validators, spec_validator_index_upper_bound())
    }

    spec fun spec_validator_indices_are_valid_addr(validators: vector<ValidatorInfo>, upper_bound: u64): bool {
        forall i in 0..len(validators):
            global<ValidatorConfig>(validators[i].addr).validator_index < upper_bound
    }

    spec fun spec_validator_indices_are_valid_config(validators: vector<ValidatorInfo>, upper_bound: u64): bool {
        forall i in 0..len(validators):
            validators[i].config.validator_index < upper_bound
    }

    spec fun spec_validator_indices_active_pending_inactive(validator_set: ValidatorSet): bool {
        len(validator_set.pending_inactive) + len(validator_set.active_validators) == spec_validator_index_upper_bound()
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
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework) || !features::spec_periodical_reward_rate_decrease_enabled();
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    // Adding helper function in staking_config leads to an unexpected error
    // So we write two helper functions here to model function staking_config::get_reward_rate().
    spec fun spec_get_reward_rate_1(config: StakingConfig): num {
        if (features::spec_periodical_reward_rate_decrease_enabled()) {
            let epoch_rewards_rate = global<staking_config::StakingRewardsConfig>(@aptos_framework).rewards_rate;
            if (epoch_rewards_rate.value == 0) {
                0
            } else {
                let denominator_0 = aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);
                let denominator = if (denominator_0 > MAX_U64) {
                    MAX_U64
                } else {
                    denominator_0
                };
                let nominator = aptos_std::fixed_point64::spec_multiply_u128(denominator, epoch_rewards_rate);
                nominator
            }
        } else {
                config.rewards_rate
        }
    }

    spec fun spec_get_reward_rate_2(config: StakingConfig): num {
        if (features::spec_periodical_reward_rate_decrease_enabled()) {
            let epoch_rewards_rate = global<staking_config::StakingRewardsConfig>(@aptos_framework).rewards_rate;
            if (epoch_rewards_rate.value == 0) {
                1
            } else {
                let denominator_0 = aptos_std::fixed_point64::spec_divide_u128(staking_config::MAX_REWARDS_RATE, epoch_rewards_rate);
                let denominator = if (denominator_0 > MAX_U64) {
                    MAX_U64
                } else {
                    denominator_0
                };
                denominator
            }
        } else {
                config.rewards_rate_denominator
        }
    }

    spec get_pending_transaction_fee {
        pragma verify = false;
    }
}
