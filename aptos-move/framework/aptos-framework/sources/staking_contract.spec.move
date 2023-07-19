spec aptos_framework::staking_contract {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec stake_pool_address(staker: address, operator: address): address {
        include StakingContractExistsAbortsIf;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec last_recorded_principal(staker: address, operator: address): u64 {
        include StakingContractExistsAbortsIf;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec commission_percentage(staker: address, operator: address): u64 {
        include StakingContractExistsAbortsIf;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) {
        // Not verified when verify_duration_estimate > vc_timeout
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved).
        let staking_contracts = global<Store>(staker).staking_contracts;
        let staking_contract = simple_map::spec_get(staking_contracts, operator);
        include StakingContractExistsAbortsIf;
        include GetStakingContractAmountsAbortsIf{staking_contract};
    }

    /// Staking_contract exists the stacker/operator pair.
    spec pending_distribution_counts(staker: address, operator: address): u64 {
        include StakingContractExistsAbortsIf;
    }

    spec staking_contract_exists(staker: address, operator: address): bool {
        aborts_if false;
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
        // TODO: complex aborts conditions in 'create_staking_contract_with_coins'.
        pragma aborts_if_is_partial;
        include WithdrawAbortsIf<AptosCoin> {account: staker};
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
        // TODO: Complex aborts conditions in 'stake::add_stake_with_cap'.
        pragma verify = false;
        pragma aborts_if_is_partial;
        aborts_if commission_percentage < 0 || commission_percentage > 100;
        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let min_stake_required = config.minimum_stake;
        let principal = coins.value;
        aborts_if principal < min_stake_required;
        let staker_address = signer::address_of(staker);
        let account = global<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address) && !exists<account::Account>(staker_address);
        aborts_if !exists<Store>(staker_address) && account.guid_creation_num + 9 > MAX_U64;
        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        aborts_if simple_map::spec_contains_key(staking_contracts, operator);
    }

    /// Account is not frozen and sufficient to withdraw.
    /// Staking_contract exists the stacker/operator pair.
    spec add_stake(staker: &signer, operator: address, amount: u64) {
        // TODO: complex aborts conditions in 'stake::add_stake_with_cap'.
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        let store = borrow_global_mut<Store>(staker_address);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let staked_coins = coin::Coin<AptosCoin>{ value: amount };
        aborts_if !exists<stake::StakePool>(staking_contract.owner_cap.pool_address);
        aborts_if staking_contract.principal + amount > MAX_U64;
        include ContractExistsAbortsIf{staker:staker_address};
        include WithdrawAbortsIf<AptosCoin>{account: staker};
    }

    /// Staking_contract exists the stacker/operator pair.
    spec update_voter(staker: &signer, operator: address, new_voter: address) {
        let staker_address = signer::address_of(staker);
        include UpdateVoterSchema {staker: staker_address};
    }

    /// Staking_contract exists the stacker/operator pair.
    /// Only active validator can update locked_until_secs.
    spec reset_lockup(staker: &signer, operator: address) {
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf{staker: staker_address};
        include IncreaseLockupWithCapAbortsIf{staker: staker_address};
    }

    spec update_commision (staker: &signer, operator: address, new_commission_percentage: u64) {
        // TODO: Call `distribute_internal` and could not verify `update_distribution_pool`.
        pragma verify = false;
    }

    /// Only staker or operator can call this.
    spec request_commission(account: &signer, staker: address, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma verify = false;
        let account_addr = signer::address_of(account);
        include ContractExistsAbortsIf{staker};
        aborts_if account_addr != staker && account_addr != operator;
    }

    spec request_commission_internal(
        operator: address,
        staking_contract: &mut StakingContract,
        add_distribution_events: &mut EventHandle<AddDistributionEvent>,
        request_commission_events: &mut EventHandle<RequestCommissionEvent>,
    ): u64 {
        // TODO: Verification timeout
        pragma verify = false;
        include GetStakingContractAmountsAbortsIf;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec unlock_rewards(staker: &signer, operator: address) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf{staker: staker_address};
    }

    spec unlock_stake(staker: &signer, operator: address, amount: u64) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma verify = false;
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator_with_same_commission(
        staker: &signer,
        old_operator: address,
        new_operator: address,
    ) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma aborts_if_is_partial;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf{staker: staker_address, operator: old_operator};
    }

    /// Staking_contract exists the stacker/operator pair.
    spec switch_operator(
        staker: &signer,
        old_operator: address,
        new_operator: address,
        new_commission_percentage: u64,
    ) {
        // TODO: Call `update_distribution_pool` and could not verify `update_distribution_pool`.
        pragma verify = false;
        let staker_address = signer::address_of(staker);
        include ContractExistsAbortsIf{staker: staker_address, operator: old_operator};
        let store = global<Store>(staker_address);
        let staking_contracts = store.staking_contracts;
        aborts_if simple_map::spec_contains_key(staking_contracts, new_operator);
    }

    /// Staking_contract exists the stacker/operator pair.
    spec distribute(staker: address, operator: address) {
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
        include GetStakingContractAmountsAbortsIf;
    }

    spec create_stake_pool(
        staker: &signer,
        operator: address,
        voter: address,
        contract_creation_seed: vector<u8>,
    ): (signer, SignerCapability, OwnerCapability) {
        // TODO: could not verify `initialize_stake_owner` because can't get `stake_pool_signer` and `stake_pool_signer_cap` from 'account::create_resource_account' function.
        pragma verify = false;
    }

    spec update_distribution_pool(
        distribution_pool: &mut Pool,
        updated_total_coins: u64,
        operator: address,
        commission_percentage: u64,
    ) {
        // TODO: complex aborts conditions in the cycle.
        pragma verify = false;
    }

    /// The Account exists under the staker.
    /// The guid_creation_num of the ccount resource is up to MAX_U64.
    spec new_staking_contracts_holder(staker: &signer): Store {
        let addr = signer::address_of(staker);
        let account = global<account::Account>(addr);
        aborts_if !exists<account::Account>(addr);
        aborts_if account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        aborts_if account.guid_creation_num + 9 > MAX_U64;
    }

    /// The Store exists under the staker.
    /// a staking_contract exists for the staker/operator pair.
    spec schema StakingContractExistsAbortsIf {
        staker: address;
        operator: address;

        aborts_if !exists<Store>(staker);
        let staking_contracts = global<Store>(staker).staking_contracts;
        aborts_if !simple_map::spec_contains_key(staking_contracts, operator);
    }

    spec schema ContractExistsAbortsIf {
        staker: address;
        operator: address;

        aborts_if !exists<Store>(staker);
        let staking_contracts = global<Store>(staker).staking_contracts;
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
        aborts_if total_active_stake < staking_contract.principal;
        aborts_if accumulated_rewards * staking_contract.commission_percentage > MAX_U64;
    }

    spec schema IncreaseLockupWithCapAbortsIf{
        use aptos_framework::timestamp;
        staker: address;
        operator: address;

        let store = global<Store>(staker);
        let staking_contract = simple_map::spec_get(store.staking_contracts, operator);
        let pool_address = staking_contract.owner_cap.pool_address;

        aborts_if !stake::stake_pool_exists(pool_address);
        aborts_if !exists<staking_config::StakingConfig>(@aptos_framework);

        let config = global<staking_config::StakingConfig>(@aptos_framework);
        let stake_pool = global<stake::StakePool>(pool_address);
        let old_locked_until_secs = stake_pool.locked_until_secs;
        let seconds =  global<timestamp::CurrentTimeMicroseconds>(@aptos_framework).microseconds / timestamp::MICRO_CONVERSION_FACTOR;
        let new_locked_until_secs =  seconds + config.recurring_lockup_duration_secs;
        aborts_if seconds + config.recurring_lockup_duration_secs > MAX_U64;
        aborts_if old_locked_until_secs > new_locked_until_secs || old_locked_until_secs == new_locked_until_secs;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }
}
