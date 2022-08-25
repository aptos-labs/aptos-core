/// Allow stakers and operators to enter a staking contract with reward sharing.
module aptos_framework::staking_contract {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_std::event::{EventHandle, emit_event};
    use aptos_std::pool_u64::{Self, Pool};
    use aptos_std::simple_map::{Self, SimpleMap};

    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::stake::{Self, OwnerCapability};
    use aptos_framework::staking_config;

    const SALT: vector<u8> = b"aptos_framework::staking_contract";

    /// Store amount must be at least the min stake required for a stake pool to join the validator set.
    const EINSUFFICIENT_STAKE_AMOUNT: u64 = 1;
    /// Commission percentage has to be between 0 and 100.
    const EINVALID_COMMISSION_PERCENTAGE: u64 = 2;
    /// Staker has no staking contracts.
    const ENO_STAKING_CONTRACT_FOUND: u64 = 3;
    /// No staking contract between the staker and operator found.
    const ESTAKING_CONTRACT_NOT_FOUND: u64 = 4;
    /// Staking contracts can't be merged.
    const ECANT_MERGE_STAKING_CONTRACTS: u64 = 5;
    /// Commission rate is zero.
    const EZERO_COMMISSION_RATE: u64 = 6;
    /// The staking contract already exists and cannot be re-created.
    const ESTAKING_CONTRACT_ALREADY_EXISTS: u64 = 7;
    /// Not enough active stake to withdraw. Some stake might still pending and will be active in the next epoch.
    const EINSUFFICIENT_ACTIVE_STAKE_TO_WITHDRAW: u64 = 8;

    /// Maximum number of distributions a stake pool can support.
    const MAXIMUM_PENDING_DISTRIBUTIONS: u64 = 20;

    struct StakingContract has store {
        // Total stake recorded after the last commission distribution.
        // This is only used to calculate the commission the operator should be receiving.
        total_stake: u64,
        pool_address: address,
        // The stake pool's owner capability. This can be used to control funds in the stake pool.
        owner_cap: OwnerCapability,
        commission_percentage: u64,
        // Current distributions, including operator commission withdrawals and staker's partial withdrawals.
        distribution_pool: Pool,
        // Just in case we need the SignerCap for stake pool account in the future.
        signer_cap: SignerCapability,
    }

    struct Store has key {
        staking_contracts: SimpleMap<address, StakingContract>,

        // Events.
        create_staking_contract_events: EventHandle<CreateStakingContractEvent>,
        update_voter_events: EventHandle<UpdateVoterEvent>,
        reset_lockup_events: EventHandle<ResetLockupEvent>,
        add_stake_events: EventHandle<AddStakeEvent>,
        request_commission_events: EventHandle<RequestCommissionEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        switch_operator_events: EventHandle<SwitchOperatorEvent>,
        add_distribution_events: EventHandle<AddDistributionEvent>,
        distribute_events: EventHandle<DistributeEvent>,
    }

    struct CreateStakingContractEvent has drop, store {
        operator: address,
        voter: address,
        pool_address: address,
        amount: u64,
        commission_percentage: u64,
    }

    struct UpdateVoterEvent has drop, store {
        operator: address,
        pool_address: address,
        old_voter: address,
        new_voter: address,
    }

    struct ResetLockupEvent has drop, store {
        operator: address,
        pool_address: address,
    }

    struct AddStakeEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64
    }

    struct RequestCommissionEvent has drop, store {
        operator: address,
        pool_address: address,
        accumulated_rewards: u64,
        commission_amount: u64,
    }

    struct UnlockStakeEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64,
        commission_paid: u64,
    }

    struct SwitchOperatorEvent has drop, store {
        old_operator: address,
        new_operator: address,
        pool_address: address,
    }

    struct AddDistributionEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64,
    }

    struct DistributeEvent has drop, store {
        operator: address,
        pool_address: address,
        recipient: address,
        amount: u64,
    }

    public fun get_stake_pool_address(staker: address, operator: address): address acquires Store {
        let staking_contracts = &borrow_global<Store>(staker).staking_contracts;
        simple_map::borrow(staking_contracts, &operator).pool_address
    }

    public fun get_last_recorded_total_stake(staker: address, operator: address): u64 acquires Store {
        let staking_contracts = &borrow_global<Store>(staker).staking_contracts;
        simple_map::borrow(staking_contracts, &operator).total_stake
    }

    public fun get_commission_percentage(staker: address, operator: address): u64 acquires Store {
        let staking_contracts = &borrow_global<Store>(staker).staking_contracts;
        simple_map::borrow(staking_contracts, &operator).commission_percentage
    }

    public fun get_staking_contract_amounts(staker: address, operator: address): (u64, u64, u64) acquires Store {
        let staking_contracts = &borrow_global<Store>(staker).staking_contracts;
        let staking_contract = simple_map::borrow(staking_contracts, &operator);
        get_staking_contract_amounts_internal(staking_contract)
    }

    /// Staker can call this function to create a simple staking contract with a specified operator.
    public entry fun create_staking_contract(
        staker: &signer,
        operator: address,
        voter: address,
        amount: u64,
        commission_percentage: u64,
    ) acquires Store {
        assert!(
            commission_percentage >= 0 && commission_percentage <= 100,
            error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE),
        );
        // The amount should be at least the min_stake_required, so the stake pool will be eligible to join the
        // validator set.
        let (min_stake_required, _) = staking_config::get_required_stake(&staking_config::get());
        assert!(amount >= min_stake_required, error::invalid_argument(EINSUFFICIENT_STAKE_AMOUNT));

        // Initialize Store resource if this is the first time the staker has delegated to anyone.
        let staker_address = signer::address_of(staker);
        if (!exists<Store>(staker_address)) {
            move_to(staker, new_staking_contracts_holder(staker));
        };

        // Cannot create the staking contract if it already exists.
        let store = borrow_global_mut<Store>(staker_address);
        let staking_contracts = &mut store.staking_contracts;
        assert!(
            !simple_map::contains_key(staking_contracts, &operator),
            error::invalid_argument(ESTAKING_CONTRACT_ALREADY_EXISTS)
        );

        // Initialize the stake pool in a new resource account. This allows the same staker to contract with multiple
        // different operators.
        let (stake_pool_signer, stake_pool_signer_cap, owner_cap) = create_stake_pool(staker, operator, voter);

        // Add the stake to the stake pool.
        let staked_coins = coin::withdraw<AptosCoin>(staker, amount);
        stake::add_stake_with_cap(&owner_cap, staked_coins);

        // Create the contract record.
        let pool_address = signer::address_of(&stake_pool_signer);
        simple_map::add(staking_contracts, operator, StakingContract {
            total_stake: amount,
            pool_address,
            owner_cap,
            commission_percentage,
            // Make sure we don't have too many pending recipients in the distribution pool.
            // Otherwise, a griefing attack is possible where the staker can keep switching operators and create too
            // many pending distributions. This can lead to out-of-gas failure whenever distribute() is called.
            distribution_pool: pool_u64::create(MAXIMUM_PENDING_DISTRIBUTIONS),
            signer_cap: stake_pool_signer_cap,
        });

        emit_event(
            &mut store.create_staking_contract_events,
            CreateStakingContractEvent { operator, voter, pool_address, amount, commission_percentage },
        );
    }

    /// Add more stake to an existing staking contract.
    public entry fun add_stake(staker: &signer, operator: address, amount: u64) acquires Store {
        let staker_address = signer::address_of(staker);
        assert_staking_contract_exists(staker_address, operator);

        let store = borrow_global_mut<Store>(staker_address);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);

        // Add the stake to the stake pool.
        let staked_coins = coin::withdraw<AptosCoin>(staker, amount);
        stake::add_stake_with_cap(&staking_contract.owner_cap, staked_coins);

        // Increase total stake (the portion belonging to the staker).
        staking_contract.total_stake = staking_contract.total_stake + amount;

        let pool_address = staking_contract.pool_address;
        emit_event(
            &mut store.add_stake_events,
            AddStakeEvent { operator, pool_address, amount },
        );
    }

    /// Convenient function to allow the staker to update the voter address in a staking contract they made.
    public entry fun update_voter(staker: &signer, operator: address, new_voter: address) acquires Store {
        let staker_address = signer::address_of(staker);
        assert_staking_contract_exists(staker_address, operator);

        let store = borrow_global_mut<Store>(staker_address);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);
        let pool_address = staking_contract.pool_address;
        let old_voter = stake::get_delegated_voter(pool_address);
        stake::set_delegated_voter_with_cap(&staking_contract.owner_cap, new_voter);

        emit_event(
            &mut store.update_voter_events,
            UpdateVoterEvent { operator, pool_address, old_voter, new_voter },
        );
    }

    /// Convenient function to allow the staker to reset their stake pool's lockup period to start now.
    public entry fun reset_lockup(staker: &signer, operator: address) acquires Store {
        let staker_address = signer::address_of(staker);
        assert_staking_contract_exists(staker_address, operator);

        let store = borrow_global_mut<Store>(staker_address);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);
        let pool_address = staking_contract.pool_address;
        stake::increase_lockup_with_cap(&staking_contract.owner_cap);

        emit_event(&mut store.reset_lockup_events, ResetLockupEvent { operator, pool_address });
    }

    /// Unlock commission amount from the stake pool. Operator needs to wait for the amount to become withdrawable
    /// at the end of the stake pool's lockup period before they can actually can withdraw_commission.
    public entry fun request_commission(operator_account: &signer, staker: address) acquires Store {
        let operator = signer::address_of(operator_account);
        assert_staking_contract_exists(staker, operator);

        let store = borrow_global_mut<Store>(staker);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);
        assert!(staking_contract.commission_percentage > 0, error::invalid_argument(EZERO_COMMISSION_RATE));

        // Force distribution of any already inactive stake.
        distribute_internal(staker, operator, staking_contract, &mut store.distribute_events);

        request_commission_internal(
            operator,
            staking_contract,
            &mut store.add_distribution_events,
            &mut store.request_commission_events,
        );
    }

    fun request_commission_internal(
        operator: address,
        staking_contract: &mut StakingContract,
        add_distribution_events: &mut EventHandle<AddDistributionEvent>,
        request_commission_events: &mut EventHandle<RequestCommissionEvent>,
    ): u64 {
        // Unlock just the commission portion from the stake pool.
        let (total_active_stake, accumulated_rewards, commission_amount) =
            get_staking_contract_amounts_internal(staking_contract);
        // Short-circuit if there's no commission to pay.
        if (commission_amount == 0) {
            return 0
        };

        // Add a distribution for the operator.
        add_distribution(operator, staking_contract, operator, commission_amount, add_distribution_events);

        // Update new total stake amount to reflect the paid commission amount.
        staking_contract.total_stake = total_active_stake - commission_amount;

        // Request to unlock the commission from the stake pool.
        // This won't become fully unlocked until the stake pool's lockup expires.
        stake::unlock_with_cap(commission_amount, &staking_contract.owner_cap);

        let pool_address = staking_contract.pool_address;
        emit_event(
            request_commission_events,
            RequestCommissionEvent { operator, pool_address, accumulated_rewards, commission_amount },
        );

        commission_amount
    }

    /// Staker can call this to request withdrawal of part or all of their staking_contract.
    /// This also triggers paying commission to the operator for accounting simplicity.
    public entry fun unlock_stake(staker: &signer, operator: address, amount: u64) acquires Store {
        let staker_address = signer::address_of(staker);
        assert_staking_contract_exists(staker_address, operator);

        let store = borrow_global_mut<Store>(staker_address);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);

        // Force distribution of any already inactive stake.
        distribute_internal(staker_address, operator, staking_contract, &mut store.distribute_events);

        // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn't
        // withdraw into the commission portion.
        let commission_paid = request_commission_internal(
            operator,
            staking_contract,
            &mut store.add_distribution_events,
            &mut store.request_commission_events,
        );

        // If there's less active stake remaining than the amount requested (potentially due to commission),
        // only withdraw up to the active amount.
        let (active, _, _, _) = stake::get_stake(staking_contract.pool_address);
        if (active < amount) {
            amount = active;
        };

        // Record a distribution for the staker.
        add_distribution(
            operator, staking_contract, staker_address, amount, &mut store.add_distribution_events);

        // Request to unlock the distribution amount from the stake pool.
        // This won't become fully unlocked until the stake pool's lockup expires.
        stake::unlock_with_cap(amount, &staking_contract.owner_cap);

        let pool_address = staking_contract.pool_address;
        emit_event(
            &mut store.unlock_stake_events,
            UnlockStakeEvent { pool_address, operator, amount, commission_paid },
        );
    }

    /// Allows staker to switch operator without going through the lenghthy process to unstake.
    public entry fun switch_operator(
        staker: &signer,
        old_operator: address,
        new_operator: address,
        new_commission_percentage: u64,
    ) acquires Store {
        let staker_address = signer::address_of(staker);
        assert_staking_contract_exists(staker_address, old_operator);

        // Merging two existing staking contracts is too complex as we'd need to merge two separate stake pools.
        let store = borrow_global_mut<Store>(staker_address);
        let staking_contracts = &mut store.staking_contracts;
        assert!(
            !simple_map::contains_key(staking_contracts, &new_operator),
            error::not_found(ECANT_MERGE_STAKING_CONTRACTS),
        );

        let (_, staking_contract) = simple_map::remove(staking_contracts, &old_operator);
        // Force distribution of any already inactive stake.
        distribute_internal(staker_address, old_operator, &mut staking_contract, &mut store.distribute_events);

        // For simplicity, we request commission to be paid out first. This avoids having to ensure to staker doesn't
        // withdraw into the commission portion.
        request_commission_internal(
            old_operator,
            &mut staking_contract,
            &mut store.add_distribution_events,
            &mut store.request_commission_events,
        );

        // Update the staking contract's commission rate and stake pool's operator.
        stake::set_operator_with_cap(&staking_contract.owner_cap, new_operator);
        staking_contract.commission_percentage = new_commission_percentage;

        let pool_address = staking_contract.pool_address;
        simple_map::add(staking_contracts, new_operator, staking_contract);
        emit_event(
            &mut store.switch_operator_events,
            SwitchOperatorEvent { pool_address, old_operator, new_operator }
        );
    }

    /// Allow anyone to distribute already unlocked funds. This does not affect reward compounding and therefore does
    /// not need to be restricted to just the staker or operator.
    public entry fun distribute(staker: address, operator: address) acquires Store {
        assert_staking_contract_exists(staker, operator);
        let store = borrow_global_mut<Store>(staker);
        let staking_contract = simple_map::borrow_mut(&mut store.staking_contracts, &operator);
        distribute_internal(staker, operator, staking_contract, &mut store.distribute_events);
    }

    /// Distribute all unlocked (inactive) funds according to distribution shares.
    fun distribute_internal(
        staker: address,
        operator:address,
        staking_contract: &mut StakingContract,
        distribute_events: &mut EventHandle<DistributeEvent>,
    ) {
        let pool_address = staking_contract.pool_address;
        let (_, inactive, _, _) = stake::get_stake(pool_address);
        // No coins to distribute. End early.
        if (inactive == 0) {
            return
        };
        let coins = stake::withdraw_with_cap(&staking_contract.owner_cap, inactive);
        let distribution_pool = &mut staking_contract.distribution_pool;
        // Update the number of coins in distribution pool to match current value.
        pool_u64::update_total_coins(distribution_pool, coin::value(&coins));

        // Buy all recipients out of the distribution pool.
        while (pool_u64::shareholders_count(distribution_pool) > 0) {
            let recipients = pool_u64::shareholders(distribution_pool);
            let recipient = *vector::borrow(&mut recipients, 0);
            let current_shares = pool_u64::shares(distribution_pool, recipient);
            let amount_to_distribute = pool_u64::redeem_shares(distribution_pool, recipient, current_shares);
            coin::deposit(recipient, coin::extract(&mut coins, amount_to_distribute));

            emit_event(
                distribute_events,
                DistributeEvent { operator, pool_address, recipient, amount: amount_to_distribute }
            );
        };

        // In case there's any dust left, send them all to the staker.
        if (coin::value(&coins) > 0) {
            coin::deposit(staker, coins);
        } else {
            coin::destroy_zero(coins);
        }
    }

    // Assert that a staking_contract exists for the staker/operator pair.
    fun assert_staking_contract_exists(staker: address, operator: address) acquires Store {
        assert!(exists<Store>(staker), error::not_found(ENO_STAKING_CONTRACT_FOUND));
        let staking_contracts = &mut borrow_global_mut<Store>(staker).staking_contracts;
        assert!(
            simple_map::contains_key(staking_contracts, &operator),
            error::not_found(ESTAKING_CONTRACT_NOT_FOUND),
        );
    }

    // Create a salt for generating the resource accounts that will be holding the StakePool for a given staking
    // contract. This address should be deterministic for the same staker and operator.
    fun create_seed(staker: address, operator: address): vector<u8> {
        let seed = bcs::to_bytes(&staker);
        vector::append(&mut seed, bcs::to_bytes(&operator));
        // Include a salt to avoid conflicts with any other modules out there that might also generate
        // deterministic resource accounts for the same staker + operator addresses.
        vector::append(&mut seed, SALT);
        seed
    }

    // Add a new distribution for `recipient` and `amount` to the staking contract's distributions list.
    fun add_distribution(
        operator: address,
        staking_contract: &mut StakingContract,
        recipient: address,
        coins_amount: u64,
        add_distribution_events: &mut EventHandle<AddDistributionEvent>,
    ) {
        // Update the total coins the pool is now worth first before adding the new distribution.
        let distribution_pool = &mut staking_contract.distribution_pool;
        let (_, _, _, total_distribution_amount) = stake::get_stake(staking_contract.pool_address);
        pool_u64::update_total_coins(distribution_pool, total_distribution_amount);
        pool_u64::buy_in(distribution_pool, recipient, coins_amount);

        let pool_address = staking_contract.pool_address;
        emit_event(
            add_distribution_events,
            AddDistributionEvent { operator, pool_address, amount: coins_amount }
        );
    }

    // Calculate accumulated rewards and commissions since last update.
    fun get_staking_contract_amounts_internal(staking_contract: &StakingContract): (u64, u64, u64) {
        // Any outgoing flows of funds before the staker withdraws all staking_contracts can only come from operator
        // withdrawing commissions.
        // So to calculate rewards, we only care about current active + pending_active - last recorded total stake.
        let (active, _, pending_active, _) = stake::get_stake(staking_contract.pool_address);
        let total_active_stake = active + pending_active;
        let accumulated_rewards = total_active_stake - staking_contract.total_stake;
        let commission_amount = accumulated_rewards * staking_contract.commission_percentage / 100;

        (total_active_stake, accumulated_rewards, commission_amount)
    }

    fun create_stake_pool(
        staker: &signer,
        operator: address,
        voter: address,
    ): (signer, SignerCapability, OwnerCapability)  {
        let seed = create_seed(signer::address_of(staker), operator);
        let (stake_pool_signer, stake_pool_signer_cap) = account::create_resource_account(staker, seed);
        // Initialize stake pool with amount = 0 as the coins need to come from the staker, not the stake pool
        // resource account.
        stake::initialize_stake_owner(&stake_pool_signer, 0, operator, voter);

        // Extract owner_cap from the StakePool, so we have control over it in the staking_contracts flow.
        // This is stored as part of the staking_contract. Thus, the staker would not have direct control over it without
        // going through well-defined functions in this module.
        let owner_cap = stake::extract_owner_cap(&stake_pool_signer);

        (stake_pool_signer, stake_pool_signer_cap, owner_cap)
    }

    // Create a new staking_contracts resource.
    fun new_staking_contracts_holder(staker: &signer): Store {
        Store {
            staking_contracts: simple_map::create<address, StakingContract>(),

            // Events.
            create_staking_contract_events: account::new_event_handle<CreateStakingContractEvent>(staker),
            update_voter_events: account::new_event_handle<UpdateVoterEvent>(staker),
            reset_lockup_events: account::new_event_handle<ResetLockupEvent>(staker),
            add_stake_events: account::new_event_handle<AddStakeEvent>(staker),
            request_commission_events: account::new_event_handle<RequestCommissionEvent>(staker),
            unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(staker),
            switch_operator_events: account::new_event_handle<SwitchOperatorEvent>(staker),
            add_distribution_events: account::new_event_handle<AddDistributionEvent>(staker),
            distribute_events: account::new_event_handle<DistributeEvent>(staker),
        }
    }

    #[test_only]
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    #[test_only]
    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    #[test_only]
    use aptos_framework::timestamp;

    #[test_only]
    const INITIAL_BALANCE: u64 = 100000000000000; // 1M APT coins with 8 decimals.

    #[test_only]
    const MAXIMUM_STAKE: u64 = 100000000000000000; // 1B APT coins with 8 decimals.

    #[test_only]
    const REWARD_BPS: u64 = 10; // 10 basis points or 0.1%

    #[test_only]
    const BPS: u64 = 10000;

    #[test_only]
    public fun setup(aptos_framework: &signer, staker: &signer, operator: &signer, initial_balance: u64) {
        // 100% per-epoch rewards for testing purposes.
        stake::initialize_for_test_custom(aptos_framework, INITIAL_BALANCE, MAXIMUM_STAKE, 1, true, REWARD_BPS, BPS, 1000000);

        account::create_account_for_test(signer::address_of(staker));
        account::create_account_for_test(signer::address_of(operator));
        stake::mint(staker, initial_balance);
        stake::mint(operator, initial_balance);

        // Let a random account join the validator set so the tested stake pool can leave the validator set later.
        let random_validator = &account::create_account_for_test(@0x42);
        stake::initialize_test_validator(random_validator, initial_balance, true, true);
    }

    #[test_only]
    const CONSENSUS_KEY: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    public fun join_validator_set(operator: &signer, pool_address: address) {
        stake::rotate_consensus_key(operator, pool_address, CONSENSUS_KEY, CONSENSUS_POP);
        stake::join_validator_set(operator, pool_address);
        stake::end_epoch();
    }

    #[test_only]
    public fun fast_forward_to_unlock() {
        timestamp::fast_forward_seconds(1);
        stake::end_epoch();
    }

    #[test_only]
    public fun setup_staking_contract(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
        amount: u64,
        commission: u64,
    ) acquires Store {
        setup(aptos_framework, staker, operator, amount);
        let operator_address = signer::address_of(operator);

        // Voter is initially set to operator but then updated to be staker.
        create_staking_contract(staker, operator_address, operator_address, amount, commission);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    public entry fun test_end_to_end(aptos_framework: &signer, staker: &signer, operator: &signer) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, INITIAL_BALANCE, 10);
        let staker_address = signer::address_of(staker);
        let operator_address = signer::address_of(operator);
        assert_staking_contract_exists(staker_address, operator_address);
        assert_staking_contract(staker_address, operator_address, INITIAL_BALANCE, 10);

        // Verify that the stake pool has been set up properly.
        let pool_address = get_stake_pool_address(staker_address, operator_address);
        stake::assert_stake_pool(pool_address, INITIAL_BALANCE, 0, 0, 0);
        assert!(get_last_recorded_total_stake(staker_address, operator_address) == INITIAL_BALANCE, 0);

        // Operator joins the validator set.
        join_validator_set(operator, pool_address);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 1);

        // Fast forward to generate rewards.
        stake::end_epoch();
        let new_balance = with_rewards(INITIAL_BALANCE);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, 0);

        // Operator claims 10% of rewards so far as commissions.
        let expected_commission_1 = (new_balance - get_last_recorded_total_stake(staker_address, operator_address)) / 10;
        new_balance = new_balance - expected_commission_1;
        request_commission(operator, staker_address);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, expected_commission_1);
        assert!(get_last_recorded_total_stake(staker_address, operator_address) == new_balance, 0);
        assert_distribution(staker_address, operator_address, operator_address, expected_commission_1);
        fast_forward_to_unlock();

        // Both original stake and operator commissions have received rewards.
        expected_commission_1 = with_rewards(expected_commission_1);
        new_balance = with_rewards(new_balance);
        stake::assert_stake_pool(pool_address, new_balance, expected_commission_1, 0, 0);
        distribute(staker_address, operator_address);
        let operator_balance = coin::balance<AptosCoin>(operator_address);
        let expected_operator_balance = INITIAL_BALANCE + expected_commission_1;
        assert!(operator_balance == expected_operator_balance, operator_balance);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, 0);
        assert_no_pending_distributions(staker_address, operator_address);

        // Staker adds more stake.
        stake::mint(staker, INITIAL_BALANCE);
        add_stake(staker, operator_address, INITIAL_BALANCE);
        stake::assert_stake_pool(pool_address, new_balance, 0, INITIAL_BALANCE, 0);

        // The newly added stake didn't receive any rewards because it was only added in the new epoch.
        stake::end_epoch();
        new_balance = with_rewards(new_balance) + INITIAL_BALANCE;

        // Second round of commission request/withdrawal.
        let expected_commission_2 = (new_balance - get_last_recorded_total_stake(staker_address, operator_address)) / 10;
        new_balance = new_balance - expected_commission_2;
        request_commission(operator, staker_address);
        assert_distribution(staker_address, operator_address, operator_address, expected_commission_2);
        assert!(get_last_recorded_total_stake(staker_address, operator_address) == new_balance, 0);
        fast_forward_to_unlock();
        expected_commission_2 = with_rewards(expected_commission_2);
        distribute(staker_address, operator_address);
        operator_balance = coin::balance<AptosCoin>(operator_address);
        expected_operator_balance = expected_operator_balance + expected_commission_2;
        assert!(operator_balance == expected_operator_balance, operator_balance);
        assert_no_pending_distributions(staker_address, operator_address);
        new_balance = with_rewards(new_balance);

        // New rounds of rewards.
        fast_forward_to_unlock();
        new_balance = with_rewards(new_balance);

        // Staker withdraws all stake, which should also request commission distribution.
        let unpaid_commission = (new_balance - get_last_recorded_total_stake(staker_address, operator_address)) / 10;
        unlock_stake(staker, operator_address, new_balance);
        stake::assert_stake_pool(pool_address, 0, 0, 0, new_balance);
        assert_distribution(staker_address, operator_address, operator_address, unpaid_commission);
        assert_distribution(staker_address, operator_address, staker_address, new_balance - unpaid_commission);

        // End epoch. The stake pool should get kicked out of the validator set as it has 0 remaining active stake.
        new_balance = with_rewards(new_balance); // With 100% rewards
        unpaid_commission = with_rewards(unpaid_commission); // With 100% rewards
        stake::end_epoch();
        stake::assert_stake_pool(pool_address, 0, new_balance, 0, 0);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE, 0);

        // Distribute and verify balances.
        distribute(staker_address, operator_address);
        assert_no_pending_distributions(staker_address, operator_address);
        operator_balance = coin::balance<AptosCoin>(operator_address);
        assert!(operator_balance == expected_operator_balance + unpaid_commission, operator_balance);
        let staker_balance = coin::balance<AptosCoin>(staker_address);
        assert!(staker_balance == new_balance - unpaid_commission, staker_balance);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10007)]
    public entry fun test_staker_cannot_create_same_staking_contract_multiple_times(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, INITIAL_BALANCE, 10);
        let operator_address = signer::address_of(operator);
        stake::mint(staker, INITIAL_BALANCE);
        create_staking_contract(staker, operator_address, operator_address, INITIAL_BALANCE, 10);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10002)]
    public entry fun test_staker_cannot_create_staking_contract_with_invalid_commission(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, INITIAL_BALANCE, 101);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    #[expected_failure(abort_code = 0x10004)]
    public entry fun test_staker_cannot_create_staking_contract_with_less_than_min_stake_required(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, 50, 100);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    public entry fun test_update_voter(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, INITIAL_BALANCE, 10);
        let staker_address = signer::address_of(staker);
        let operator_address = signer::address_of(operator);

        // Voter is initially set to operator but then updated to be staker.
        let pool_address = get_stake_pool_address(staker_address, operator_address);
        assert!(stake::get_delegated_voter(pool_address) == operator_address, 0);
        update_voter(staker, operator_address, staker_address);
        assert!(stake::get_delegated_voter(pool_address) == staker_address, 1);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    public entry fun test_reset_lockup(
        aptos_framework: &signer,
        staker: &signer,
        operator: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator, INITIAL_BALANCE, 10);
        let staker_address = signer::address_of(staker);
        let operator_address = signer::address_of(operator);
        let pool_address = get_stake_pool_address(staker_address, operator_address);

        let origin_lockup_expiration = stake::get_lockup_secs(pool_address);
        reset_lockup(staker, operator_address);
        assert!(origin_lockup_expiration < stake::get_lockup_secs(pool_address), 0);
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator_1 = @0x234, operator_2 = @0x345)]
    public entry fun test_staker_can_switch_operator(
        aptos_framework: &signer,
        staker: &signer,
        operator_1: &signer,
        operator_2: &signer,
    ) acquires Store {
        setup_staking_contract(aptos_framework, staker, operator_1, INITIAL_BALANCE, 10);
        account::create_account_for_test(signer::address_of(operator_2));
        stake::mint(operator_2, INITIAL_BALANCE);
        let staker_address = signer::address_of(staker);
        let operator_1_address = signer::address_of(operator_1);
        let operator_2_address = signer::address_of(operator_2);

        // Join validator set and earn some rewards.
        let pool_address = get_stake_pool_address(staker_address, operator_1_address);
        join_validator_set(operator_1, pool_address);
        stake::end_epoch();
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 0);

        // Switch operators.
        switch_operator(staker, operator_1_address, operator_2_address, 20);
        // The staking_contract is now associated with operator 2 but there should be a pending distribution of unpaid
        // commission to operator 1.
        let new_balance = with_rewards(INITIAL_BALANCE);
        let commission_for_operator_1 = (new_balance - INITIAL_BALANCE) / 10;
        assert_distribution(staker_address, operator_2_address, operator_1_address, commission_for_operator_1);
        // Unpaid commission should be unlocked from the stake pool.
        new_balance = new_balance - commission_for_operator_1;
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, commission_for_operator_1);
        assert!(get_last_recorded_total_stake(staker_address, operator_2_address) == new_balance, 0);

        // The stake pool's validator should not have left the validator set.
        assert!(get_stake_pool_address(staker_address, operator_2_address) == pool_address, 1);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 2);

        // End epoch to get more rewards.
        fast_forward_to_unlock();
        new_balance = with_rewards(new_balance);
        commission_for_operator_1 = with_rewards(commission_for_operator_1);

        // Verify that when commissions are withdrawn, previous pending distribution to operator 1 also happens.
        // Then new commission of 20% is paid to operator 2.
        let commission_for_operator_2 =
            (new_balance - get_last_recorded_total_stake(staker_address, operator_2_address)) / 5;
        new_balance = new_balance - commission_for_operator_2;
        request_commission(operator_2, staker_address);
        assert_distribution(staker_address, operator_2_address, operator_2_address, commission_for_operator_2);
        let operator_1_balance = coin::balance<AptosCoin>(operator_1_address);
        assert!(operator_1_balance == INITIAL_BALANCE + commission_for_operator_1, operator_1_balance);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, commission_for_operator_2);
        assert!(get_last_recorded_total_stake(staker_address, operator_2_address) == new_balance, 0);
        fast_forward_to_unlock();

        // Operator 2's commission is distributed.
        distribute(staker_address, operator_2_address);
        let operator_2_balance = coin::balance<AptosCoin>(operator_2_address);
        new_balance = with_rewards(new_balance);
        commission_for_operator_2 = with_rewards(commission_for_operator_2);
        assert!(operator_2_balance == INITIAL_BALANCE + commission_for_operator_2, operator_2_balance);
        stake::assert_stake_pool(
            pool_address,
            new_balance,
            0,
            0,
            0,
        );
    }

    #[test(aptos_framework = @0x1, staker = @0x123, operator = @0x234)]
    public entry fun test_staker_can_withdraw_partial_stake(
        aptos_framework: &signer, staker: &signer, operator: &signer) acquires Store {
        let initial_balance = INITIAL_BALANCE * 2;
        setup_staking_contract(aptos_framework, staker, operator, initial_balance, 10);
        let staker_address = signer::address_of(staker);
        let operator_address = signer::address_of(operator);
        let pool_address = get_stake_pool_address(staker_address, operator_address);

        // Operator joins the validator set so rewards are generated.
        join_validator_set(operator, pool_address);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 1);

        // Fast forward to generate rewards.
        stake::end_epoch();
        let new_balance = with_rewards(initial_balance);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, 0);

        // Staker withdraws 1/4 of the stake, which should also request commission distribution.
        let withdrawn_stake = new_balance / 4;
        let unpaid_commission = (new_balance - initial_balance) / 10;
        let new_balance = new_balance - withdrawn_stake - unpaid_commission;
        unlock_stake(staker, operator_address, withdrawn_stake);
        stake::assert_stake_pool(pool_address, new_balance, 0, 0, withdrawn_stake + unpaid_commission);
        assert_distribution(staker_address, operator_address, operator_address, unpaid_commission);
        assert_distribution(staker_address, operator_address, staker_address, withdrawn_stake);

        // The validator is still in the active set as its remaining stake is still above min required.
        stake::end_epoch();
        new_balance = with_rewards(new_balance);
        unpaid_commission = with_rewards(unpaid_commission);
        withdrawn_stake = with_rewards(withdrawn_stake);
        stake::assert_stake_pool(pool_address, new_balance, withdrawn_stake + unpaid_commission, 0, 0);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 0);

        // Distribute and verify balances.
        distribute(staker_address, operator_address);
        assert_no_pending_distributions(staker_address, operator_address);
        let operator_balance = coin::balance<AptosCoin>(operator_address);
        assert!(operator_balance == initial_balance + unpaid_commission, operator_balance);
        let staker_balance = coin::balance<AptosCoin>(staker_address);
        assert!(staker_balance == withdrawn_stake, staker_balance);
    }

    #[test_only]
    public fun assert_staking_contract(
        staker: address, operator: address, total_stake: u64, commission_percentage: u64) acquires Store {
        let staking_contract = simple_map::borrow(&borrow_global<Store>(staker).staking_contracts, &operator);
        assert!(staking_contract.total_stake == total_stake, staking_contract.total_stake);
        assert!(staking_contract.commission_percentage == commission_percentage, staking_contract.commission_percentage);
    }

    #[test_only]
    public fun assert_no_pending_distributions(staker: address, operator: address) acquires Store {
        let staking_contract = simple_map::borrow(&borrow_global<Store>(staker).staking_contracts, &operator);
        let shareholders_count = pool_u64::shareholders_count(&staking_contract.distribution_pool);
        assert!(shareholders_count == 0, shareholders_count);
    }

    #[test_only]
    public fun assert_distribution(
        staker: address, operator: address, recipient: address, coins_amount: u64) acquires Store {
        let staking_contract = simple_map::borrow(&borrow_global<Store>(staker).staking_contracts, &operator);
        let distribution_balance = pool_u64::balance(&staking_contract.distribution_pool, recipient);
        assert!(distribution_balance == coins_amount, distribution_balance);
    }

    #[test_only]
    fun with_rewards(amount: u64): u64 {
        amount + amount * REWARD_BPS / BPS
    }
}
