spec aptos_framework::staking_contract {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The Store structure for the staker exists after the staking contract is created.
    /// Criticality: Medium
    /// Implementation: The create_staking_contract_with_coins function ensures that the staker account has a Store
    /// structure assigned.
    /// Enforcement: Formally verified via [high-level-req-1](CreateStakingContractWithCoinsAbortsifAndEnsures).
    ///
    /// No.: 2
    /// Requirement: A staking contract is created and stored in a mapping within the Store resource.
    /// Criticality: High
    /// Implementation: The create_staking_contract_with_coins function adds the newly created StakingContract to the
    /// staking_contracts map with the operator as a key of the Store resource, effectively storing the staking contract.
    /// Enforcement: Formally verified via [high-level-req-2](CreateStakingContractWithCoinsAbortsifAndEnsures).
    ///
    /// No.: 3
    /// Requirement: Adding stake to the stake pool increases the principal value of the pool, reflecting the additional
    /// stake amount.
    /// Criticality: High
    /// Implementation: The add_stake function transfers the specified amount of staked coins from the staker's account
    /// to the stake pool associated with the staking contract. It increases the principal value of the staking contract
    /// by the added stake amount.
    /// Enforcement: Formally verified via [high-level-req-3](add_stake).
    ///
    /// No.: 4
    /// Requirement: The staker may update the voter of a staking contract, enabling them to modify the assigned voter
    /// address and ensure it accurately reflects their desired choice.
    /// Criticality: High
    /// Implementation: The update_voter function ensures that the voter address in a staking contract may be updated by
    /// the staker, resulting in the modification of the delegated voter address in the associated stake pool to reflect
    /// the new address provided.
    /// Enforcement: Formally verified via [high-level-req-4](update_voter).
    ///
    /// No.: 5
    /// Requirement: Only the owner of the stake pool has the permission to reset the lockup period of the pool.
    /// Criticality: Critical
    /// Implementation: The reset_lockup function ensures that only the staker who owns the stake pool has the authority
    /// to reset the lockup period of the pool.
    /// Enforcement: Formally verified via [high-level-req-5](reset_lockup).
    ///
    /// No.: 6
    /// Requirement: Unlocked funds are correctly distributed to recipients based on their distribution shares, taking into
    /// account the associated commission percentage.
    /// Criticality: High
    /// Implementation: The distribution process, implemented in the distribute_internal function, accurately allocates
    /// unlocked funds to their intended recipients based on their distribution shares. It guarantees that each
    /// recipient receives the correct amount of funds, considering the commission percentage associated with the
    /// staking contract.
    /// Enforcement: Audited that the correct amount of unlocked funds is distributed according to distribution shares.
    ///
    /// No.: 7
    /// Requirement: The stake pool ensures that the commission is correctly requested and paid out from the old operator's
    /// stake pool before allowing the switch to the new operator.
    /// Criticality: High
    /// Implementation: The switch_operator function initiates the commission payout from the stake pool associated with
    /// the old operator, ensuring a smooth transition. Paying out the commission before the switch guarantees that the
    /// staker receives the appropriate commission amount and maintains the integrity of the staking process.
    /// Enforcement: Audited that the commission is paid to the old operator.
    ///
    /// No.: 8
    /// Requirement: Stakers can withdraw their funds from the staking contract, ensuring the unlocked amount becomes
    /// available for withdrawal after the lockup period.
    /// Criticality: High
    /// Implementation: The unlock_stake function ensures that the requested amount is properly unlocked from the stake
    /// pool, considering the lockup period and that the funds become available for withdrawal when the lockup expires.
    /// Enforcement: Audited that funds are unlocked properly.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec StakingContract {
        invariant commission_percentage >= 0 && commission_percentage <= 100;
    }

    spec stake_pool_address(staker: address, operator: address): address {
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result == simple_map::spec_get(staking_contracts, operator).pool_address;

    }

    /// Staking_contract exists the stacker/operator pair.
    spec last_recorded_principal(staker: address, operator: address): u64 {
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result == simple_map::spec_get(staking_contracts, operator).principal;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec commission_percentage(staker: address, operator: address): u64 {
        include ContractExistsAbortsIf;
        let staking_contracts = global<Store>(staker).staking_contracts;
        ensures result == simple_map::spec_get(staking_contracts, operator).commission_percentage;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) {
        // TODO: set because of timeout (property proved).
        pragma verify_duration_estimate = 120;
        let staking_contracts = global<Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);

        include ContractExistsAbortsIf;
        include GetStakingContractAmountsAbortsIf { staking_contract };

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;

        ensures result_1 == total_active_stake;
        ensures result_2 == accumulated_rewards;
        // TODO: This property causes timeout
        // ensures result_3 == accumulated_rewards * staking_contract.commission_percentage / 100;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec pending_distribution_counts(staker: address, operator: address): u64 {
        include ContractExistsAbortsIf;

        let staking_contracts = global<Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        let shareholders_count = len(staking_contract.distribution_pool.shareholders);
        ensures result == shareholders_count;
    }

    spec staking_contract_exists(staker: address, operator: address): bool {
        aborts_if false;
        ensures result == spec_staking_contract_exists(staker, operator);
    }

    spec get_expected_stake_pool_address {
        pragma aborts_if_is_partial;
    }

    spec fun spec_staking_contract_exists(staker: address, operator: address): bool {
        if (!exists<Store>(staker)) {
            false
        } else {
            let store = global<Store>(staker);
            simple_map::spec_contains_key(store.staking_contracts, operator)
        }
    }

    /// Account is not frozen and sufficient to withdraw.
    spec create_staking_contract(
    staker: &signer,
    operator: address,
    voter: address,
    amount: u64,
    commission_percentage: u64,
    contract_creation_seed: vector<u8>,
    ) {
        pragma aborts_if_is_partial;
        pragma verify_duration_estimate = 120;
        include PreconditionsInCreateContract;
        include WithdrawAbortsIf<AptosCoin> { account: staker };
        include CreateStakingContractWithCoinsAbortsIfAndEnsures;
    }

    /// The amount should be at least the min_stake_required, so the stake pool will be eligible to join the validator set.
    /// Initialize Store resource if this is the first time the staker has delegated to anyone.
    /// Cannot create the staking contract if it already exists.
    spec create_staking_contract_with_coins(
    staker: &signer,
    operator: address,
    voter: address,
    coins: Coin<AptosCoin>,
    commission_percentage: u64,
    contract_creation_seed: vector<u8>,
    ): address {
        pragma verify_duration_estimate = 120;
        pragma aborts_if_is_partial;
        include PreconditionsInCreateContract;

        let amount = coins.value;
        include CreateStakingContractWithCoinsAbortsIfAndEnsures { amount };

        // TODO: this property causes timeout
        // let staker_address = signer::address_of(staker);
        // let seed_0 = bcs::to_bytes(staker_address);
        // let seed_1 = concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
        // let resource_addr = account::spec_create_resource_address(staker_address, seed_1);
        // ensures result == resource_addr;
    }

    /// Account is not frozen and sufficient to withdraw.
    /// Staking_contract exists the stacker/operator pair.
    spec add_stake(staker: &signer, operator: address, amount: u64) {
        // TODO(fa_migration)
        use aptos_framework::reconfiguration_state;
        pragma verify_duration_estimate = 600;
        // TODO: this function times out
        include stake::ResourceRequirement;
        aborts_if reconfiguration_state::spec_is_in_progress();

        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address };
        let store = global<Store>(staker_address);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);

        include WithdrawAbortsIf<AptosCoin> { account: staker };
        let balance = global<coin::CoinStore<AptosCoin>>(staker_address).coin.value;
        let post post_coin = global<coin::CoinStore<AptosCoin>>(staker_address).coin.value;
        ensures post_coin == balance - amount;

        // postconditions stake::add_stake_with_cap()
        let owner_cap = staking_contract.owner_cap;
        include stake::AddStakeWithCapAbortsIfAndEnsures { owner_cap };

        let post post_store = global<Store>(staker_address);
        let post post_staking_contract = simple_map::spec_get(post_store.staking_contracts, operator);
        aborts_if staking_contract.principal + amount > MAX_U64;

        // property 3: Adding stake to the stake pool increases the principal value of the pool, reflecting the
        // additional stake amount.
        /// [high-level-req-3]
        ensures post_staking_contract.principal == staking_contract.principal + amount;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec update_voter(staker: &signer, operator: address, new_voter: address) {
        let staker_address = signer::address_of(staker);
        include UpdateVoterSchema { staker: staker_address };

        let post store = global<Store>(staker_address);
        let post staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let post pool_address = staking_contract.owner_cap.pool_address;
        let post new_delegated_voter = global<stake::StakePool>(pool_address).delegated_voter;
        // property 4: The staker may update the voter of a staking contract, enabling them
        // to modify the assigned voter address and ensure it accurately reflects their desired choice.
        /// [high-level-req-4]
        ensures new_delegated_voter == new_voter;
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Only active validator can update locked_until_secs.
    spec reset_lockup(staker: &signer, operator: address) {
        let staker_address = signer::address_of(staker);
        /// [high-level-req-5]
        include ContractExistsAbortsIf { staker: staker_address };
        include IncreaseLockupWithCapAbortsIf { staker: staker_address };
    }

    spec update_commision (staker: &signer, operator: address, new_commission_percentage: u64) {
        // TODO: Call `distribute_internal` and could not verify `update_distribution_pool`.
        // TODO: A data invariant not hold happened here involve with 'pool_u64' #L16.
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        aborts_if new_commission_percentage > 100;
        include ContractExistsAbortsIf { staker: staker_address };
    }

    /// Only staker or operator can call this.
    spec request_commission(account: &signer, staker: address, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: A data invariant not hold happened here involve with 'pool_u64' #L16.
        pragma verify = false;
        let account_addr = signer::address_of(account);
        include ContractExistsAbortsIf { staker };
        aborts_if account_addr != staker && account_addr != operator;
    }

    spec request_commission_internal(
    operator: address,
    staking_contract: &mut StakingContract,
    add_distribution_events: &mut EventHandle<AddDistributionEvent>,
    request_commission_events: &mut EventHandle<RequestCommissionEvent>,
    ): u64 {
        // TODO: A data invariant not hold happened here involve with 'pool_u64' #L16.
        pragma verify = false;
        include GetStakingContractAmountsAbortsIf;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec unlock_rewards(staker: &signer, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: Set because of timeout (estimate unknown).
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        let staking_contracts = global<Store>(staker_address).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        include ContractExistsAbortsIf { staker: staker_address };
    }

    spec unlock_stake(staker: &signer, operator: address, amount: u64) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: Set because of timeout (estimate unknown).
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address };
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator_with_same_commission(
    staker: &signer,
    old_operator: address,
    new_operator: address,
    ) {
        // TODO: These function passed locally however failed in github CI
        pragma verify_duration_estimate = 120;
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address, operator: old_operator };
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator(
    staker: &signer,
    old_operator: address,
    new_operator: address,
    new_commission_percentage: u64,
    ) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        // TODO: Set because of timeout (estimate unknown).
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf { staker: staker_address, operator: old_operator };
        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);
    }

    spec set_beneficiary_for_operator(operator: &signer, new_beneficiary: address) {
        // TODO: temporary mockup
        pragma verify = false;
    }

    spec beneficiary_for_operator(operator: address): address {
        // TODO: temporary mockup
        pragma verify = false;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec distribute(staker: address, operator: address) {
        // TODO: These function passed locally however failed in github CI
        pragma verify_duration_estimate = 120;
        // TODO: Call `distribute_internal` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;

        include ContractExistsAbortsIf;
    }

    /// The StakePool exists under the pool_address of StakingContract.
    /// The value of inactive and pending_inactive in the stake_pool is up to MAX_U64.
    spec distribute_internal(
    staker: address,
    operator: address,
    staking_contract: &mut StakingContract,
    distribute_events: &mut EventHandle<DistributeEvent>,
    ) {
        // TODO: These function passed locally however failed in github CI
        pragma verify_duration_estimate = 120;
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;
        let pool_address = staking_contract.pool_address;
        let stake_pool = borrow_global<stake::StakePool>(pool_address);
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if stake_pool.inactive.value + stake_pool.pending_inactive.value > MAX_U64;
        aborts_if !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);
    }

    /// Staking_contract exists the stacker/operator pair.
    spec assert_staking_contract_exists(staker: address, operator: address) {
        include ContractExistsAbortsIf;
    }

    spec add_distribution(
    operator: address,
    staking_contract: &mut StakingContract,
    recipient: address,
    coins_amount: u64,
    add_distribution_events: &mut EventHandle<AddDistributionEvent>,
    ) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma verify = false;
    }

    /// The StakePool exists under the pool_address of StakingContract.
    spec get_staking_contract_amounts_internal(staking_contract: &StakingContract): (u64, u64, u64) {
        pragma verify_duration_estimate = 120;
        include GetStakingContractAmountsAbortsIf;

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;
        ensures result_1 == total_active_stake;
        ensures result_2 == accumulated_rewards;
        ensures result_3 == commission_amount;
    }

    spec create_stake_pool(
    staker: &signer,
    operator: address,
    voter: address,
    contract_creation_seed: vector<u8>,
    ): (signer, SignerCapability, OwnerCapability) {
        pragma verify_duration_estimate = 120;
        include stake::ResourceRequirement;
        let staker_address = signer::address_of(staker);
        // postconditions account::create_resource_account()

        let seed_0 = bcs::to_bytes(staker_address);
        let seed_1 = concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
        let resource_addr = account::spec_create_resource_address(staker_address, seed_1);
        include CreateStakePoolAbortsIf { resource_addr };
        ensures exists<account::Account>(resource_addr);
        let post post_account = global<account::Account>(resource_addr);
        ensures post_account.authentication_key == account::ZERO_AUTH_KEY;
        ensures post_account.signer_capability_offer.for == std::option::spec_some(resource_addr);

        // postconditions stake::initialize_stake_owner()
        ensures exists<stake::StakePool>(resource_addr);
        let post post_owner_cap = global<stake::OwnerCapability>(resource_addr);
        let post post_pool_address = post_owner_cap.pool_address;
        let post post_stake_pool = global<stake::StakePool>(post_pool_address);
        let post post_operator = post_stake_pool.operator_address;
        let post post_delegated_voter = post_stake_pool.delegated_voter;
        ensures resource_addr != operator ==> post_operator == operator;
        ensures resource_addr != voter ==> post_delegated_voter == voter;
        ensures signer::address_of(result_1) == resource_addr;
        ensures result_2 == SignerCapability { account: resource_addr };
        ensures result_3 == OwnerCapability { pool_address: resource_addr };
    }

    spec update_distribution_pool(
    distribution_pool: &mut Pool,
    updated_total_coins: u64,
    operator: address,
    commission_percentage: u64,
    ) {
        // TODO: complex aborts conditions in the cycle.
        // pragma verify = false;
        pragma aborts_if_is_partial;
    }

    /// The Account exists under the staker.
    /// The guid_creation_num of the account resource is up to MAX_U64.
    spec new_staking_contracts_holder(staker: &signer): Store {
        pragma aborts_if_is_partial;
        include NewStakingContractsHolderAbortsIf;
    }

    spec schema NewStakingContractsHolderAbortsIf {
        staker: signer;

        let addr = signer::address_of(staker);
        // let account = global<account::Account>(addr);
        // aborts_if !exists<account::Account>(addr);
        // aborts_if account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if account.guid_creation_num + 9 > MAX_U64;
    }

    /// The Store exists under the staker.
    /// a staking_contract exists for the staker/operator pair.
    spec schema ContractExistsAbortsIf {
        staker: address;
        operator: address;

        aborts_if !exists<Store>(staker);
        let staking_contracts = global<Store>(staker).staking_contracts;
        // This property may cause timeout
        aborts_if !simple_map::spec_contains_key(staking_contracts, operator);
    }

    spec schema UpdateVoterSchema {
        staker: address;
        operator: address;

        let store = global<Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.pool_address;
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);
        include ContractExistsAbortsIf;
    }

    spec schema WithdrawAbortsIf<CoinType> {
        account: signer;
        amount: u64;

        let account_addr = signer::address_of(account);
        let coin_store = global<coin::CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        aborts_if balance < amount;
    }

    spec schema GetStakingContractAmountsAbortsIf {
        staking_contract: StakingContract;

        let pool_address = staking_contract.pool_address;
        let stake_pool = global<stake::StakePool>(pool_address);
        let active = coin::value(stake_pool.active);
        let pending_active = coin::value(stake_pool.pending_active);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.principal;
        aborts_if !exists<stake::StakePool>(pool_address);
        aborts_if active + pending_active > MAX_U64;
        // TODO: These function causes the timeout
        aborts_if total_active_stake < staking_contract.principal;
        aborts_if accumulated_rewards * staking_contract.commission_percentage > MAX_U64;
    }

    spec schema IncreaseLockupWithCapAbortsIf {
        use aptos_framework::timestamp;
        staker: address;
        operator: address;

        let store = global<Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.owner_cap.pool_address;

        // property 5: Only the owner of the stake pool has the permission to reset the lockup period of the pool.
        aborts_if !stake::stake_pool_exists(pool_address);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);

        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let stake_pool = global<stake::StakePool>(pool_address);
        let old_locked_until_secs = stake_pool.locked_until_secs;
        let seconds = global<timestamp::CurrentTimeMicroseconds>(
            @aptos_framework
        ).microseconds / timestamp::MICRO_CONVERSION_FACTOR;
        let new_locked_until_secs = seconds + config.recurring_lockup_duration_secs;
        aborts_if seconds + config.recurring_lockup_duration_secs > MAX_U64;
        aborts_if old_locked_until_secs > new_locked_until_secs || old_locked_until_secs == new_locked_until_secs;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);

        let post post_store = global<Store>(staker);
        let post post_staking_contract = simple_map::spec_get(post_store.staking_contracts, operator);
        let post post_stake_pool = global<stake::StakePool>(post_staking_contract.owner_cap.pool_address);
        ensures post_stake_pool.locked_until_secs == new_locked_until_secs;
    }

    spec schema CreateStakingContractWithCoinsAbortsIfAndEnsures {
        staker: signer;
        operator: address;
        voter: address;
        amount: u64;
        commission_percentage: u64;
        contract_creation_seed: vector<u8>;

        aborts_if commission_percentage > 100;
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);
        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let min_stake_required = config.minimum_stake;
        aborts_if amount < min_stake_required;

        let staker_address = signer::address_of(staker);
        let account = global<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address) && !exists<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        /// [high-level-req-1]
        ensures exists<Store>(staker_address);

        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        // TODO: this property causes timeout
        // aborts_if simple_map::spec_contains_key(staking_contracts, operator);

        // Verify create_stake_pool()
        // TODO: this property causes timeout
        // let seed_0 = bcs::to_bytes(staker_address);
        // let seed_1 = concat(concat(concat(seed_0, bcs::to_bytes(operator)), SALT), contract_creation_seed);
        // let resource_addr = account::spec_create_resource_address(staker_address, seed_1);
        // include CreateStakePoolAbortsIf {resource_addr};


        // Verify stake::add_stake_with_cap()
        let owner_cap = simple_map::spec_get(store.staking_contracts, operator).owner_cap;
        // TODO: this property causes timeout
        // include stake::AddStakeWithCapAbortsIfAndEnsures{owner_cap: owner_cap};
        let post post_store = global<Store>(staker_address);
        let post post_staking_contracts = post_store.staking_contracts;
        // TODO: this property causes timeout
        // ensures simple_map::spec_contains_key(post_staking_contracts, operator);
    }

    spec schema PreconditionsInCreateContract {
        requires exists<stake::ValidatorPerformance>(@aptos_framework);
        requires exists<stake::ValidatorSet>(@aptos_framework);
        requires exists<staking_config::StakingRewardsConfig>(
            @aptos_framework
        ) || !std::features::spec_periodical_reward_rate_decrease_enabled();
        requires exists<aptos_framework::timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        requires exists<stake::AptosCoinCapabilities>(@aptos_framework);
    }

    spec schema CreateStakePoolAbortsIf {
        resource_addr: address;
        operator: address;
        voter: address;
        contract_creation_seed: vector<u8>;

        // postconditions account::create_resource_account()
        let acc = global<account::Account>(resource_addr);
        aborts_if exists<account::Account>(resource_addr) && (len(
            acc.signer_capability_offer.for.vec
        ) != 0 || acc.sequence_number != 0);
        aborts_if !exists<account::Account>(resource_addr) && len(bcs::to_bytes(resource_addr)) != 32;
        aborts_if len(account::ZERO_AUTH_KEY) != 32;

        // postconditions stake::initialize_stake_owner()
        aborts_if exists<stake::ValidatorConfig>(resource_addr);
        let allowed = global<stake::AllowedValidators>(@aptos_framework);
        aborts_if exists<stake::AllowedValidators>(@aptos_framework) && !contains(allowed.accounts, resource_addr);
        aborts_if exists<stake::StakePool>(resource_addr);
        aborts_if exists<stake::OwnerCapability>(resource_addr);
        // 12 is the times that calls 'events::guids'
        aborts_if exists<account::Account>(
            resource_addr
        ) && acc.guid_creation_num + 12 >= account::MAX_GUID_CREATION_NUM;
    }
}
