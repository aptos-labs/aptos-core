/// Simple vesting contract that allows specifying how much coins should be vesting in each fixed-size period.
/// The vesting contract also supports staking and reward distribtion.
module aptos_framework::vesting {
    use std::bcs;
    use std::error;
    use std::fixed_point32::{Self, FixedPoint32};
    use std::signer;
    use std::vector;

    use aptos_std::event::{EventHandle, emit_event};
    use aptos_std::simple_map::{Self, SimpleMap};

    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::staking_config;
    use aptos_framework::stake::{Self, OwnerCapability};
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    friend aptos_framework::genesis;

    const VESTING_POOL_SALT: vector<u8> = b"aptos_framework::vesting";

    /// Vesting amount must be at least the min stake required for a stake pool to join the validator set.
    const EINSUFFICIENT_AMOUNT: u64 = 1;
    /// Withdrawal address is invalid.
    const EINVALID_WITHDRAWAL_ADDRESS: u64 = 2;
    /// Vesting schedule cannot be empty.
    const EEMPTY_VESTING_SCHEDULE: u64 = 3;

    /// Validator status enum copied from aptos_framework::stake.
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;
    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Vesting pool states.
    /// Vesting pool is active and distributions can be made.
    const VESTING_POOL_ACTIVE: u64 = 1;
    /// Vesting pool has been paused by the admin and no distributions can be made.
    const VESTING_POOL_PAUSED: u64 = 2;
    /// Vesting pool has been terminated and all funds have been released back to the withdrawal address.
    const VESTING_POOL_TERMINATED: u64 = 3;
    /// Vesting pool has ended as all funds have been distributed.
    const VESTING_POOL_ENDED: u64 = 4;
    /// Shareholders list cannot be empty.
    const ENO_SHAREHOLDERS: u64 = 5;
    /// Vesting cannot start before or at the current block timestamp. Has to be in the future.
    const EVESTING_START_TOO_SOON: u64 = 6;
    /// The signer is not the admin of the vesting pool.
    const ENOT_ADMIN: u64 = 7;
    /// The length of shareholders and shares lists don't match.
    const ESHARES_LENGTH_MISMATCH: u64 = 8;
    /// Vesting pool needs to be in active state.
    const EVESTING_POOL_NOT_ACTIVE: u64 = 9;
    /// Vesting pool not in paused state.
    const EVESTING_POOL_NOT_PAUSED: u64 = 10;
    /// Cannot terminate a vesting pool if there are any in-flight distributions.
    const EDISTRIBUTIONS_IN_FLIGHT: u64 = 11;
    /// Cannot terminate a vesting pool if there's stake in pending_active state. Admin can wait until the next network
    /// epoch to terminate.
    const EPENDING_ACTIVE_STAKE: u64 = 12;
    /// Can only withdraw from terminated vesting pools.
    const EVESTING_POOL_NOT_TERMINATED: u64 = 13;
    /// There's no unlocked stake to withdraw from the stake pool.
    const ENO_STAKE_TO_WITHDRAW: u64 = 14;
    /// Vesting has not started yet. Need to wait until the specified vesting start timestamp.
    const EVESTING_HAS_NOT_STARTED: u64 = 15;
    /// No new vesting period has passed since the last distribution.
    const ENO_NEW_VESTING_PERIOD_HAS_PASSED: u64 = 16;
    /// There are no vested coins and rewards to distribute.
    const ENO_COINS_TO_DISTRIBUTE: u64 = 17;
    /// Shareholder not found in specified vesting pool.
    const ESHAREHOLDER_NOT_FOUND: u64 = 18;

    struct VestingPool has key {
        state: u64,
        admin: address,
        beneficiaries: SimpleMap<address, address>,
        shareholders: vector<address>,
        shares: SimpleMap<address, u64>,
        // Total amount at the start of the vesting pool.
        grant_amount: u64,
        // Withdrawal address where all funds would be released back to if the admin terminates the vesting contract.
        // This is for security purposes in case the admin account is different from where funds should be held.
        withdrawal_address: address,
        // The owner cap for the stake pool where the unvested funds will be staked at.
        stake_pool_owner_cap: OwnerCapability,
        // Where the vesting pool is located at. Included for convenience.
        pool_address: address,

        // The vesting schedule as a list of fractions that vest for each period. The last number is repeated until the
        // vesting amount runs out.
        // For example [1/24, 1/24, 1/48] with a period of 1 month means that after vesting starts, the first two months
        // will vest 1/24 of the original total amount. From the third month only, 1/48 will vest until the vesting fund
        // runs out.
        vesting_schedule: vector<FixedPoint32>,
        // When the vesting should start.
        vesting_start_timestamp_secs: u64,
        // How long each vesting period is. For example 1 month.
        vesting_period_duration: u64,
        // Last vesting period, 1-indexed. For example if 2 months have passed, the last vesting period, if distribution
        // was requested, would be 2. Default value is 0 which means there have been no vesting periods yet.
        last_vesting_period: u64,

        // Not used currently but might in the future if this module is upgraded. We want to keep the signer cap just
        // in case.
        signer_cap: SignerCapability,
    }

    /// Resource holding a nonce that's stored at the admin account. We'll use this to create resource accounts for new
    /// vesting pools so there's no address collision.
    struct AdminNonce has key {
        vesting_pool_creation_nonce: u64,
    }

    public fun get_vesting_pool_state(pool_address: address): u64 acquires VestingPool {
        borrow_global<VestingPool>(pool_address).state
    }

    public fun get_beneficiary(pool_address: address, shareholder: address): address acquires VestingPool {
        let beneficiaries = &borrow_global<VestingPool>(pool_address).beneficiaries;
        *simple_map::borrow(beneficiaries, &shareholder)
    }

    public fun get_shares(pool_address: address, shareholder: address): u64 acquires VestingPool {
        let shares = &borrow_global<VestingPool>(pool_address).shares;
        *simple_map::borrow(shares, &shareholder)
    }

    public fun get_grant_amount(pool_address: address): u64 acquires VestingPool {
        borrow_global<VestingPool>(pool_address).grant_amount
    }

    public fun get_vesting_schedule(pool_address: address): vector<FixedPoint32> acquires VestingPool {
        borrow_global<VestingPool>(pool_address).vesting_schedule
    }

    public fun get_vesting_period_duration(pool_address: address): u64 acquires VestingPool {
        borrow_global<VestingPool>(pool_address).vesting_period_duration
    }

    public fun get_last_vesting_period(pool_address: address): u64 acquires VestingPool {
        borrow_global<VestingPool>(pool_address).last_vesting_period
    }

    public fun get_vesting_start_timestamp(pool_address: address): u64 acquires VestingPool {
        borrow_global<VestingPool>(pool_address).vesting_start_timestamp_secs
    }

    public fun get_withdrawal_address(pool_address: address): address acquires VestingPool {
        borrow_global<VestingPool>(pool_address).withdrawal_address
    }

    /// Delegator can call this function to define a simple delegation contract with a specified operator.
    ///
    /// Can only delegate to a specific operator once. Afterward, delegator cannot update amount or commission.
    public fun create_vesting_account(
        admin: &signer,
        grant_amount: u64,
        shareholders: vector<address>,
        shares: vector<u64>,
        vesting_schedule: vector<FixedPoint32>,
        withdrawal_address: address,
        vesting_start_timestamp_secs: u64,
        vesting_period_duration: u64,
        operator: address,
        voter: address,
    ) acquires AdminNonce {
        validate_vesting_account_inputs(
            grant_amount,
            &shareholders,
            &shares,
            &vesting_schedule,
            withdrawal_address,
            vesting_start_timestamp_secs,
        );

        // If this is the first time this admin account has created a vesting pool, initialize the nonce.
        let admin_address = signer::address_of(admin);
        if (!exists<AdminNonce>(admin_address)) {
            move_to(admin, AdminNonce { vesting_pool_creation_nonce: 0 });
        };

        // Initialize the vesting pool in a new resource account. This allows the same admin to create multiple pools.
        let (vesting_pool_account_signer, vesting_pool_account_signer_cap) = create_vesting_pool_account(admin);
        stake::initialize_stake_owner(&vesting_pool_account_signer, 0, operator, voter);

        // Extract owner_cap from the StakePool, so we have control over it in the delegations flow.
        // This is stored as part of the delegation. Thus, the delegator would not have direct control over it without
        // going through well-defined functions in this module.
        let stake_pool_owner_cap = stake::extract_owner_cap(&vesting_pool_account_signer);

        // Add the unvested amount to the stake pool.
        let unvested_coins = coin::withdraw<AptosCoin>(admin, grant_amount);
        stake::add_stake_with_cap(&stake_pool_owner_cap, unvested_coins);

        move_to(&vesting_pool_account_signer, VestingPool {
            state: VESTING_POOL_ACTIVE,
            admin: admin_address,
            grant_amount,
            beneficiaries: simple_map::create<address, address>(),
            shareholders,
            shares: create_shares_map(&shareholders, &shares),
            withdrawal_address,
            stake_pool_owner_cap,
            pool_address: signer::address_of(&vesting_pool_account_signer),
            signer_cap: vesting_pool_account_signer_cap,

            vesting_schedule,
            vesting_start_timestamp_secs,
            vesting_period_duration,
            last_vesting_period: 0,
        });
    }

    public entry fun batch_request_distribution(admin: &signer, pool_addresses: vector<address>) acquires VestingPool {
        let len = vector::length(&pool_addresses);
        let i = 0;
        while (i < len) {
            request_distribution(admin, *vector::borrow(&pool_addresses, i));
            i = i + 1;
        };
    }

    public entry fun request_distribution(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert_active_vesting_pool(vesting_pool);

        // Ensure vesting has started.
        let now_seconds = timestamp::now_seconds();
        let vesting_start_timestamp_secs = vesting_pool.vesting_start_timestamp_secs;
        assert!(
            now_seconds >= vesting_start_timestamp_secs,
            error::invalid_state(EVESTING_HAS_NOT_STARTED),
        );

        // Ensure at least one full vesting period has passed since the last distribution.
        let vesting_period_duration = vesting_pool.vesting_period_duration;
        // If 2 full periods have passed and we're in period 3 for example, last_complete_vesting_period would be 2.
        let last_complete_vesting_period = (now_seconds - vesting_start_timestamp_secs) / vesting_period_duration;
        // This means that we can only request distribution if last_vesting_period < 2, i.e. we have not distruted for
        // period 2 yet.
        assert!(
            vesting_pool.last_vesting_period < last_complete_vesting_period,
            error::invalid_state(ENO_NEW_VESTING_PERIOD_HAS_PASSED),
        );
        // We'll do distribution for last_vesting_period + 1. This way, we don't skip any vesting periods.
        // For example, if last_vesting_period is 1 but last_complete_vesting_period is 3, we'll do distribution for
        // period 2 first. Admin can call request_distribution() once more afterward to do distribution for period 3.
        let period_to_distribute = vesting_pool.last_vesting_period + 1;
        vesting_pool.last_vesting_period = vesting_pool.last_vesting_period + 1;

        // Calculate how much has vested, excluding rewards.
        let vesting_schedule = &vesting_pool.vesting_schedule;
        // Index is 0-based while period is 1-based so we need to subtract 1.
        let vesting_schedule_index = period_to_distribute - 1;
        let vesting_fraction = if (vesting_schedule_index < vector::length(vesting_schedule)) {
            *vector::borrow(vesting_schedule, vesting_schedule_index)
        } else {
            // Last vesting schedule fraction will repeat until the vesting fund runs out.
            *vector::borrow(vesting_schedule, vector::length(vesting_schedule) - 1)
        };
        let grant_amount = vesting_pool.grant_amount;
        let vested_amount = fixed_point32::multiply_u64(grant_amount, vesting_fraction);

        // Calculate how much reward has accumulated
        let (active_stake, _, _ , _) = stake::get_stake(pool_address);
        let accumulated_rewards = active_stake - grant_amount;

        // Request unlock the vested amount + rewards from the stake pool.
        let total_amount_to_distribute = vested_amount + accumulated_rewards;
        assert!(total_amount_to_distribute > 0, error::invalid_state(ENO_COINS_TO_DISTRIBUTE));
        stake::unlock_with_cap(total_amount_to_distribute, &vesting_pool.stake_pool_owner_cap);
    }

    /// Distribute any withdrawable stake from the stake pool.
    public entry fun distribute(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert_active_vesting_pool(vesting_pool);

        let (_, total_withdrawable, _, _) = stake::get_stake(pool_address);
        let coins_to_distribute =
            stake::withdraw_with_cap(&vesting_pool.stake_pool_owner_cap, total_withdrawable);
        let distribute_amount = (coin::value(&coins_to_distribute) as u128);

        let len = vector::length(&vesting_pool.shareholders);
        let i = 0;
        let total_shares = (vesting_pool.grant_amount as u128);
        while (i < len) {
            let shareholder = *vector::borrow(&vesting_pool.shareholders, i);
            let shares = (*simple_map::borrow(&vesting_pool.shares, &shareholder) as u128);
            // u128 math to avoid overflow from multiplication. Then convert back to u64 after division.
            // Multiplication first to minimize rounding error.
            let amount = ((distribute_amount * shares / total_shares) as u64);
            let coins = coin::extract(&mut coins_to_distribute, amount);
            coin::deposit(shareholder, coins);

            i = i + 1;
        };

        // Send any remaining "dust" (leftover due to rounding error) to the withdrawal address.
        coin::deposit(vesting_pool.withdrawal_address, coins_to_distribute);
    }

    public entry fun pause_vesting_pool(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert_active_vesting_pool(vesting_pool);

        vesting_pool.state = VESTING_POOL_PAUSED;
    }

    public entry fun unpause_vesting_pool(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert!(vesting_pool.state == VESTING_POOL_PAUSED, error::invalid_state(EVESTING_POOL_NOT_PAUSED));

        vesting_pool.state = VESTING_POOL_ACTIVE;
    }

    public entry fun terminate_vesting_pool(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert_active_vesting_pool(vesting_pool);

        // Cannot terminate the vesting pool if there's an in-flight distribution.
        let (_, withdrawable, _, _pending_unlocked) = stake::get_stake(pool_address);
        assert!(withdrawable + _pending_unlocked == 0, error::invalid_state(EDISTRIBUTIONS_IN_FLIGHT));

        vesting_pool.state = VESTING_POOL_TERMINATED;

        // Request unlock of all stake.
        let (active, _, pending_active, _) = stake::get_stake(pool_address);
        assert!(pending_active == 0, error::invalid_state(EPENDING_ACTIVE_STAKE));
        stake::unlock_with_cap(active, &vesting_pool.stake_pool_owner_cap);
    }

    public entry fun withdraw_from_terminated_pool(admin: &signer, pool_address: address) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        assert!(vesting_pool.state == VESTING_POOL_TERMINATED, error::invalid_state(EVESTING_POOL_NOT_TERMINATED));

        // Withdraw any fully unlocked stake.
        let (_, withdrawable_amount, _, _) = stake::get_stake(pool_address);
        assert!(withdrawable_amount > 0, error::invalid_state(ENO_STAKE_TO_WITHDRAW));
        let coins = stake::withdraw_with_cap(&vesting_pool.stake_pool_owner_cap, withdrawable_amount);
        coin::deposit(vesting_pool.withdrawal_address, coins);
    }

    public entry fun update_operator(
        admin: &signer,
        pool_address: address,
        new_operator: address,
    ) acquires VestingPool {
        let vesting_pool = borrow_global<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        stake::set_operator_with_cap(&vesting_pool.stake_pool_owner_cap, new_operator);
    }

    public entry fun update_voter(
        admin: &signer,
        pool_address: address,
        new_voter: address,
    ) acquires VestingPool {
        let vesting_pool = borrow_global<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);
        stake::set_delegated_voter_with_cap(&vesting_pool.stake_pool_owner_cap, new_voter);
    }

    public entry fun update_beneficiary(
        admin: &signer,
        pool_address: address,
        shareholder: address,
        new_beneficiary: address,
    ) acquires VestingPool {
        let vesting_pool = borrow_global_mut<VestingPool>(pool_address);
        verify_admin(admin, vesting_pool);

        let beneficiaries = &mut vesting_pool.beneficiaries;
        assert!(simple_map::contains_key(beneficiaries, &shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        let beneficiary = simple_map::borrow_mut(beneficiaries, &shareholder);
        *beneficiary = new_beneficiary;
    }

    // Create a salt for generating the resource accounts that will be holding the VestingPool.
    // This address should be deterministic for the same admin and vesting pool creation nonce.
    fun create_vesting_pool_account(admin: &signer): (signer, SignerCapability) acquires AdminNonce {
        let admin_nonce = borrow_global_mut<AdminNonce>(signer::address_of(admin));
        let seed = bcs::to_bytes(admin);
        vector::append(&mut seed, bcs::to_bytes(&admin_nonce.vesting_pool_creation_nonce));
        admin_nonce.vesting_pool_creation_nonce = admin_nonce.vesting_pool_creation_nonce + 1;

        // Include a salt to avoid conflicts with any other modules out there that might also generate
        // deterministic resource accounts for the same admin address + nonce.
        vector::append(&mut seed, VESTING_POOL_SALT);
        account::create_resource_account(admin, seed)
    }

    fun validate_vesting_account_inputs(
        amount: u64,
        shareholders: &vector<address>,
        shares: &vector<u64>,
        vesting_schedule: &vector<FixedPoint32>,
        withdrawal_address: address,
        vesting_start_timestamp_secs: u64,
    ) {
        // The delegated stake should be at least the min_stake_required, so the stake pool will be eligible to join the
        // validator set.
        let (min_stake_required, _) = staking_config::get_required_stake(&staking_config::get());
        assert!(amount > min_stake_required, error::invalid_argument(EINSUFFICIENT_AMOUNT));

        assert!(
            !system_addresses::is_reserved_address(withdrawal_address),
            error::invalid_argument(EINVALID_WITHDRAWAL_ADDRESS),
        );
        assert!(vector::length(vesting_schedule) > 0, error::invalid_argument(EEMPTY_VESTING_SCHEDULE));
        assert!(
            vector::length(shares) == vector::length(shareholders),
            error::invalid_argument(ESHARES_LENGTH_MISMATCH),
        );
        assert!(vector::length(shareholders) > 0, error::invalid_argument(ENO_SHAREHOLDERS));
        assert!(
            vesting_start_timestamp_secs > timestamp::now_seconds(),
            error::invalid_argument(EVESTING_START_TOO_SOON),
        );
    }

    fun verify_admin(admin: &signer, vesting_pool: &VestingPool) {
        assert!(signer::address_of(admin) == vesting_pool.admin, error::unauthenticated(ENOT_ADMIN));
    }

    fun assert_active_vesting_pool(vesting_pool: &VestingPool) {
        assert!(vesting_pool.state == VESTING_POOL_ACTIVE, error::invalid_state(EVESTING_POOL_NOT_ACTIVE));
    }

    fun create_shares_map(shareholders: &vector<address>, shares: &vector<u64>): SimpleMap<address, u64> {
        let shares_map = simple_map::create<address ,u64>();
        let len = vector::length(shareholders);
        let i = 0;
        while (i < len) {
            simple_map::add(&mut shares_map,*vector::borrow(shareholders, i), *vector::borrow(shares, i));
            i = i + 1;
        };

        shares_map
    }
}
