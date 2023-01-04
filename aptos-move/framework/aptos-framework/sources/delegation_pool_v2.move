module aptos_framework::delegate {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_std::math64::min;

    use aptos_framework::delegation_pool::{
    Self,
    get_stake_pool_signer,
    current_lockup_epoch,
    buy_in_active_shares,
    buy_in_inactive_shares,
    redeem_active_shares,
    redeem_inactive_shares,
    pending_withdrawal_exists,
    };

    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::stake;
    use aptos_framework::timestamp;

    const SALT: vector<u8> = b"aptos_framework::delegate";

    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Delegation pool owner capability does not exist at the provided account.
    const EOWNER_CAP_NOT_FOUND: u64 = 1;

    /// Account is already owning a delegation pool.
    const EOWNER_CAP_ALREADY_EXISTS: u64 = 2;

    const MAX_U64: u64 = 18446744073709551615;

    /// Capability that represents ownership over not-shared operations of underlying stake pool.
    struct DelegationPoolOwnership has key, store {
        /// equal to address of the resource account owning the stake pool
        pool_address: address,
    }

    public entry fun initialize_delegation_pool(owner: &signer, delegation_pool_creation_seed: vector<u8>) {
        let owner_address = signer::address_of(owner);
        assert!(!owner_cap_exists(owner_address), error::already_exists(EOWNER_CAP_ALREADY_EXISTS));

        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = bcs::to_bytes(&owner_address);
        // include a salt to avoid conflicts with any other modules creating resource accounts
        vector::append(&mut seed, SALT);
        // include an additional salt in case the same resource account has already been created.
        vector::append(&mut seed, delegation_pool_creation_seed);

        let (stake_pool_signer, stake_pool_signer_cap) = account::create_resource_account(owner, seed);
        coin::register<AptosCoin>(&stake_pool_signer);

        // stake_pool_signer is owner account of stake pool and has `OwnerCapability`
        let pool_address = signer::address_of(&stake_pool_signer);
        stake::initialize_stake_owner(&stake_pool_signer, 0, owner_address, owner_address);

        delegation_pool::initialize(&stake_pool_signer, stake_pool_signer_cap);

        // save resource-account address (inner pool address) + outer pool ownership on `owner`
        move_to(owner, DelegationPoolOwnership { pool_address });
    }

    public fun owner_cap_exists(addr: address): bool {
        exists<DelegationPoolOwnership>(addr)
    }

    fun assert_owner_cap_exists(owner: address) {
        assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
    }

    public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership {
        assert_owner_cap_exists(owner);
        borrow_global<DelegationPoolOwnership>(owner).pool_address
    }

    public entry fun set_operator(owner: &signer, new_operator: address) acquires DelegationPoolOwnership {
        stake::set_operator(
            &get_stake_pool_signer(get_owned_pool_address(signer::address_of(owner))),
            new_operator
        );
    }

    public entry fun set_delegated_voter(owner: &signer, new_voter: address) acquires DelegationPoolOwnership {
        stake::set_delegated_voter(
            &get_stake_pool_signer(get_owned_pool_address(signer::address_of(owner))),
            new_voter
        );
    }

    public entry fun add_stake(delegator: &signer, pool_address: address, amount: u64) {
        let stake_pool_signer = get_stake_pool_signer(pool_address);
        let delegator_address = signer::address_of(delegator);

        coin::transfer<AptosCoin>(delegator, signer::address_of(&stake_pool_signer), amount);
        stake::add_stake(&stake_pool_signer, amount);

        amount = delegation_pool::charge_add_stake_fee(pool_address, amount);
        buy_in_active_shares(pool_address, delegator_address, amount);

        delegation_pool::emit_add_stake_event(pool_address, delegator_address, amount);
    }

    public entry fun unlock(delegator: &signer, pool_address: address, amount: u64) {
        // execute pending withdrawal if existing before creating a new one
        withdraw(delegator, pool_address, MAX_U64);

        let stake_pool_signer = get_stake_pool_signer(pool_address);
        let delegator_address = signer::address_of(delegator);

        // ensure there is enough active stake on stake pool to unlock
        let (active, _, _, _) = stake::get_stake(pool_address);
        let amount = min(amount, active);

        amount = redeem_active_shares(pool_address, delegator_address, amount);
        stake::unlock(&stake_pool_signer, amount);
        buy_in_inactive_shares(pool_address, delegator_address, amount);

        delegation_pool::emit_unlock_stake_event(pool_address, delegator_address, amount);
    }

    public entry fun reactivate_stake(delegator: &signer, pool_address: address, amount: u64) {
        let stake_pool_signer = get_stake_pool_signer(pool_address);
        let delegator_address = signer::address_of(delegator);

        let amount = redeem_inactive_shares(pool_address, delegator_address, amount, current_lockup_epoch(pool_address));
        stake::reactivate_stake(&stake_pool_signer, amount);
        buy_in_active_shares(pool_address, delegator_address, amount);

        delegation_pool::emit_reactivate_stake_event(pool_address, delegator_address, amount);
    }

    public entry fun withdraw(delegator: &signer, pool_address: address, amount: u64) {
        let stake_pool_signer = get_stake_pool_signer(pool_address);
        let delegator_address = signer::address_of(delegator);

        let (withdrawal_exists, withdrawal_lockup_epoch) = pending_withdrawal_exists(pool_address, delegator_address);
        if (!(
            withdrawal_exists &&
            (
                withdrawal_lockup_epoch < current_lockup_epoch(pool_address) ||
                (
                    stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE &&
                    timestamp::now_seconds() >= stake::get_lockup_secs(pool_address)
                )
            )
        )) { return };

        let amount = redeem_inactive_shares(pool_address, delegator_address, amount, withdrawal_lockup_epoch);
        stake::withdraw(&stake_pool_signer, amount);
        coin::transfer<AptosCoin>(&stake_pool_signer, delegator_address, amount);

        delegation_pool::emit_withdraw_stake_event(pool_address, delegator_address, amount);
    }

    #[test_only]
    use aptos_framework::reconfiguration;

    #[test_only]
    const CONSENSUS_KEY_1: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP_1: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    const EPOCH_DURATION: u64 = 60;
    #[test_only]
    const LOCKUP_CYCLE_SECONDS: u64 = 3600;

    #[test_only]
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    #[test_only]
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    #[test_only]
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;

    #[test_only]
    public fun end_aptos_epoch() {
        stake::end_epoch(); // additionally fast-forwards `EPOCH_DURATION` seconds
        reconfiguration::reconfigure_for_test_custom();
    }

    #[test_only]
    public fun initialize_for_test(aptos_framework: &signer) {
        initialize_for_test_custom(aptos_framework, 100, 10000, LOCKUP_CYCLE_SECONDS, true, 1, 100, 1000000);
    }

    #[test_only]
    public fun initialize_for_test_custom(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate_numerator: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        account::create_account_for_test(signer::address_of(aptos_framework));
        stake::initialize_for_test_custom(
            aptos_framework,
            minimum_stake,
            maximum_stake,
            recurring_lockup_secs,
            allow_validator_set_change,
            rewards_rate_numerator,
            rewards_rate_denominator,
            voting_power_increase_limit,
        );
        reconfiguration::initialize_for_test(aptos_framework);
    }

    #[test_only]
    public fun initialize_test_validator(
        validator: &signer,
        amount: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
    ) acquires DelegationPoolOwnership {
        let validator_address = signer::address_of(validator);
        if (!account::exists_at(validator_address)) {
            account::create_account_for_test(validator_address);
        };

        initialize_delegation_pool(validator, vector::empty<u8>());

        // validator is initially stake pool's operator and voter
        let pool_address = get_owned_pool_address(validator_address);
        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);

        if (amount > 0) {
            stake::mint(validator, amount);
            add_stake(validator, pool_address, amount);
        };

        if (should_join_validator_set) {
            stake::join_validator_set(validator, pool_address);
        };

        if (should_end_epoch) {
            end_aptos_epoch();
        };
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_set_operator_and_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);

        assert!(stake::get_operator(pool_address) == @0x123, 1);
        assert!(stake::get_delegated_voter(pool_address) == @0x123, 1);

        set_operator(validator, @0x111);
        assert!(stake::get_operator(pool_address) == @0x111, 2);

        set_delegated_voter(validator, @0x112);
        assert!(stake::get_delegated_voter(pool_address) == @0x112, 2);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_operator(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        // account does not own any delegation pool
        set_operator(validator, @0x111);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        // account does not own any delegation pool
        set_delegated_voter(validator, @0x112);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x80002, location = Self)]
    public entry fun test_already_owns_delegation_pool(
        aptos_framework: &signer,
        validator: &signer,
    ) {
        initialize_for_test(aptos_framework);
        initialize_delegation_pool(validator, x"00");
        initialize_delegation_pool(validator, x"01");
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_initialize_delegation_pool(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);

        assert!(stake::stake_pool_exists(pool_address), 1);
        assert!(stake::get_operator(pool_address) == validator_address, 2);
        assert!(stake::get_delegated_voter(pool_address) == validator_address, 3);

        delegation_pool::assert_delegation_pool_exists(pool_address);
        assert_owner_cap_exists(validator_address);
        assert!(borrow_global<DelegationPoolOwnership>(validator_address).pool_address == pool_address, 4);

        let (consensus_pubkey, network_addresses, fullnode_addresses) = stake::get_validator_config(pool_address);
        assert!(consensus_pubkey == vector::empty<u8>(), 7);
        assert!(network_addresses == vector::empty<u8>(), 8);
        assert!(fullnode_addresses == vector::empty<u8>(), 9);

        assert!(delegation_pool::current_lockup_epoch(pool_address) == 0, 10);
        stake::assert_stake_pool(pool_address, 0, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_add_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, false, false);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // stake pool is pending active => all stake is activated by default
        stake::assert_stake_pool(pool_address, 1000, 0, 0, 0);
        assert_delegation(validator_address, pool_address, 1000, 0, 0);

        // check `add_stake` increases delegator and stake pool active balances
        stake::mint(validator, 250);
        add_stake(validator, pool_address, 250);
        // zero `add_stake` fee as pool will not produce rewards this epoch
        assert_delegation(validator_address, pool_address, 1250, 0, 0);
        // added stake is still activated directly on the stake pool
        stake::assert_stake_pool(pool_address, 1250, 0, 0, 0);

        // move validator to active state
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // add 250 coins being pending active until next epoch
        stake::mint(validator, 250);
        add_stake(validator, pool_address, 250);
        stake::assert_stake_pool(pool_address, 1250, 0, 250, 0);
        // zero `add_stake` fee as there is only one delegator which gets back the paid fee
        // as entire active stake is owned by itself
        assert_delegation(validator_address, pool_address, 1500, 0, 0);

        // add 100 additional coins being pending active until next epoch
        stake::mint(validator, 100);
        add_stake(validator, pool_address, 100);
        stake::assert_stake_pool(pool_address, 1250, 0, 350, 0);
        assert_delegation(validator_address, pool_address, 1600, 0, 0);

        end_aptos_epoch();
        // 1250 active stake (* 1% rewards) + 350 pending active stake
        assert_delegation(validator_address, pool_address, 1612, 0, 0);
        stake::assert_stake_pool(pool_address, 1612, 0, 0, 0);

        // add stake which should produce no rewards yet
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);

        end_aptos_epoch();
        // 1612 active stake (* 1% rewards) + 200 pending active stake
        assert_delegation(validator_address, pool_address, 1828, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_add_stake_many(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::assert_stake_pool(pool_address, 1000, 0, 0, 0);

        // add 250 coins from another account
        stake::mint(delegator, 250);
        add_stake(delegator, pool_address, 250);
        stake::assert_stake_pool(pool_address, 1000, 0, 250, 0);
        // after `add_stake` fee: 250 * 1000 / (1000 + 1000 active * 1%) = 247
        assert_delegation(delegator_address, pool_address, 247, 0, 0);

        end_aptos_epoch();
        // 1000 active stake * 1% rewards
        stake::assert_stake_pool(pool_address, 1260, 0, 0, 0);

        // 3 `add_stake` fee + 1000 active stake * 1% = 13 total rewards
        // 13 * 247 / (1000 + 247) = 2
        assert_delegation(delegator_address, pool_address, 249, 0, 0);
        // 13 * 1000 / (1000 + 247) = 10
        assert_delegation(validator_address, pool_address, 1010, 0, 0);

        stake::mint(validator, 250);
        add_stake(validator, pool_address, 250);
        // after `add_stake` fee: 250 * 1260 / (1260 + 1260 active * 1%) = 247
        // from 3 `add_stake` fee 2 distributed to `validator` and 1 to `delegator`
        assert_delegation(validator_address, pool_address, 1259, 0, 0);
        assert_delegation(delegator_address, pool_address, 250, 0, 0);
        stake::assert_stake_pool(pool_address, 1260, 0, 250, 0);

        stake::mint(delegator, 100);
        add_stake(delegator, pool_address, 100);
        // after `add_stake` fee: 100 * 1510 / (1510 + 1260 active * 1%) = 99
        assert_delegation(delegator_address, pool_address, 348, 0, 0);
        assert_delegation(validator_address, pool_address, 1261, 0, 0);
        stake::assert_stake_pool(pool_address, 1260, 0, 350, 0);

        end_aptos_epoch();
        // 249 active stake * 1% rewards + 100 pending active
        assert_delegation(delegator_address, pool_address, 351, 0, 0);
        // 1010 active stake * 1% rewards + 250 pending active
        assert_delegation(validator_address, pool_address, 1270, 0, 0);
        stake::assert_stake_pool(pool_address, 1622, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_unlock_single(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 100, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add 200 coins being pending active until next epoch
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 300, 0, 0);
        stake::assert_stake_pool(pool_address, 100, 0, 200, 0);

        // cannot unlock pending active stake of pool (99/150 stake goes to pending inactive)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 201, 0, 99);
        stake::assert_stake_pool(pool_address, 1, 0, 200, 99);

        // reactivate entire pending inactive stake
        reactivate_stake(validator, pool_address, 100);
        assert_delegation(validator_address, pool_address, 300, 0, 0);
        stake::assert_stake_pool(pool_address, 100, 0, 200, 0);

        end_aptos_epoch();
        // 100 active stake * 1% rewards + 200 pending active
        assert_delegation(validator_address, pool_address, 301, 0, 0);

        // once stake becomes active can unlock any amount of it (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 152, 0, 149);
        stake::assert_stake_pool(pool_address, 152, 0, 0, 149);

        assert!(stake::get_remaining_lockup_secs(pool_address) == LOCKUP_CYCLE_SECONDS - EPOCH_DURATION, 1);
        end_aptos_epoch(); // forwards EPOCH_DURATION seconds

        // pending inactive stake should not be inactivated yet
        // 152 active stake * 1% + 149 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 153, 0, 150);
        stake::assert_stake_pool(pool_address, 153, 0, 0, 150);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 2 * EPOCH_DURATION);
        end_aptos_epoch(); // forwards another EPOCH_DURATION and unlocks stake

        // 153 active stake * 1% + 150 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 154, 151, 0);

        // add 50 coins from another account
        stake::mint(delegator, 50);
        add_stake(delegator, pool_address, 50);
        // after `add_stake` fee: 50 * 153 / (153 + 153 active * 1%) = 49
        assert_delegation(delegator_address, pool_address, 48, 0, 0);
        assert_delegation(validator_address, pool_address, 155, 151, 0);
        stake::assert_stake_pool(pool_address, 154, 151, 50, 0);

        // try to withdraw stake unlocked by others
        withdraw(delegator, pool_address, 50);
        assert!(coin::balance<AptosCoin>(delegator_address) == 0, 1);

        // withdraw own unlocked stake
        withdraw(validator, pool_address, 200);
        assert!(coin::balance<AptosCoin>(validator_address) == 151, 2);
        assert_delegation(validator_address, pool_address, 155, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_reactivate_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // unlock some stake from the active one
        unlock(validator, pool_address, 100);
        assert_delegation(validator_address, pool_address, 100, 0, 100);
        stake::assert_stake_pool(pool_address, 100, 0, 0, 100);

        // add some stake in pending active state
        stake::mint(validator, 150);
        add_stake(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 250, 0, 100);
        stake::assert_stake_pool(pool_address, 100, 0, 150, 100);

        // can reactivate only pending inactive stake
        reactivate_stake(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 350, 0, 0);
        stake::assert_stake_pool(pool_address, 200, 0, 150, 0);

        end_aptos_epoch();
        // 200 active stake * 1% rewards + 150 pending active
        assert_delegation(validator_address, pool_address, 352, 0, 0);

        // unlock stake added at previous epoch (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 203, 0, 149);
        stake::assert_stake_pool(pool_address, 203, 0, 0, 149);

        // inactivate the pending inactive stake
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 2 * EPOCH_DURATION);
        end_aptos_epoch();
        // 203 active stake * 1% rewards and 149 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 205, 150, 0);

        // try to unlock stake in a new lockup cycle (pending withdrawal is executed)
        unlock(validator, pool_address, 100);
        assert!(coin::balance<AptosCoin>(validator_address) == 150, 1);
        assert_delegation(validator_address, pool_address, 106, 0, 99);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_active_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        // 1000 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 1010, 0, 0);

        // add stake in pending_active state
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 1210, 0, 0);

        end_aptos_epoch();
        // 1010 active stake * 1% rewards and 200 pending_active stake with no rewards yet
        assert_delegation(validator_address, pool_address, 1220, 0, 0);
        end_aptos_epoch();

        // 1220 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 1232, 0, 0);

        // 1232 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1244 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1256 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1268 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1280 active stake * 1% rewards == 12
        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 1292, 0, 0);

        // schedule some coins for unlocking (expect some imprecision when moving shares)
        unlock(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 1093, 0, 199);

        // end this lockup cycle
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // 1093 active stake * 1% rewards and 199 pending_inactive * 1% rewards
        assert_delegation(validator_address, pool_address, 1103, 200, 0);

        end_aptos_epoch();
        // 1103 active stake * 1% rewards and 0 from inactive stake
        assert_delegation(validator_address, pool_address, 1114, 200, 0);

        // add stake in pending_active state
        stake::mint(validator, 1000);
        add_stake(validator, pool_address, 1000);
        assert_delegation(validator_address, pool_address, 2114, 200, 0);

        end_aptos_epoch();
        // 1114 active stake * 1% rewards and 0 from inactive stake
        assert_delegation(validator_address, pool_address, 2125, 200, 0);

        end_aptos_epoch();
        // 2125 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 2146, 200, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_active_stake_rewards_multiple(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add stake in pending_active state
        stake::mint(delegator, 300);
        add_stake(delegator, pool_address, 300);
        // after `add_stake` fee: 300 * 200 / (200 + 200 active * 1%) = 297
        // 3 `add_stake` fee * 297 / (200 + 297) = 1
        assert_delegation(delegator_address, pool_address, 298, 0, 0);
        // 3 `add_stake` fee * 200 / (200 + 297) = 1
        assert_delegation(validator_address, pool_address, 201, 0, 0);
        stake::assert_stake_pool(pool_address, 200, 0, 300, 0);

        // 200 active stake * 1% rewards are shared between active and pending_active stakes for this epoch only
        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 299, 0, 0);
        assert_delegation(validator_address, pool_address, 202, 0, 0);
        stake::assert_stake_pool(pool_address, 502, 0, 0, 0);

        // from now delegators earn their own rewards
        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 302, 0, 0);
        assert_delegation(validator_address, pool_address, 204, 0, 0);
        stake::assert_stake_pool(pool_address, 507, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 305, 0, 0);
        assert_delegation(validator_address, pool_address, 206, 0, 0);
        stake::assert_stake_pool(pool_address, 512, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 308, 0, 0);
        assert_delegation(validator_address, pool_address, 208, 0, 0);
        stake::assert_stake_pool(pool_address, 517, 0, 0, 0);

        // add more stake in pending_active state than currently active
        stake::mint(delegator, 1000);
        add_stake(delegator, pool_address, 1000);
        // after `add_stake` fee: 1000 * 517 / (517 + 517 active * 1%) = 990
        // 10 `add_stake` fee * (308 + 990) / (308 + 990 + 208) ~ 9
        assert_delegation(delegator_address, pool_address, 1307, 0, 0);
        // 10 `add_stake` fee * 208 / (308 + 990 + 208) ~ 1
        assert_delegation(validator_address, pool_address, 209, 0, 0);

        end_aptos_epoch();
        // delegator should have at most 308 + 308 active * 1% + 1000 stake
        assert_delegation(delegator_address, pool_address, 1311, 0, 0);
        assert_delegation(validator_address, pool_address, 210, 0, 0);
        stake::assert_stake_pool(pool_address, 1522, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_pending_inactive_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 1010, 0, 0);

        // schedule some coins for unlocking (expect some imprecision when moving shares)
        unlock(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 811, 0, 199);

        end_aptos_epoch(); // 811 * 1% active and 199 * 1% pending inactive
        end_aptos_epoch(); // 819 * 1% active and 200 * 1% pending inactive

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 827 * 1% active and 202 * 1% pending inactive
        end_aptos_epoch(); // 835 * 1% active and 0 pending inactive (inactivated)
        assert_delegation(validator_address, pool_address, 843, 204, 0);

        // triggers an automatic execution of the unlocked pending withdraw
        unlock(validator, pool_address, 200);
        assert!(coin::balance<AptosCoin>(validator_address) == 204, 1);
        assert_delegation(validator_address, pool_address, 644, 0, 199);

        end_aptos_epoch(); // 644 * 1% active and 199 * 1% pending inactive
        end_aptos_epoch(); // 650 * 1% active and 200 * 1% pending inactive
        end_aptos_epoch(); // 656 * 1% active and 202 * 1% pending inactive
        end_aptos_epoch(); // 662 * 1% active and 204 * 1% pending inactive
        // the lockup cycle is not ended, pending inactive stake still earning
        assert_delegation(validator_address, pool_address, 668, 0, 206);
    }

    #[test_only]
    public fun assert_delegation(
        delegator_address: address,
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_inactive_stake: u64,
    ) {
        let (actual_active, actual_inactive, actual_pending_inactive) = delegation_pool::get_stake(pool_address, delegator_address);
        assert!(actual_active == active_stake, actual_active);
        assert!(actual_inactive == inactive_stake, actual_inactive);
        assert!(actual_pending_inactive == pending_inactive_stake, actual_pending_inactive);
    }
}
