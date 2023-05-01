/**
 * Allow multiple delegators to participate in the same stake pool in order to collect the minimum
 * stake required to join the validator set. Delegators are rewarded out of the validator rewards
 * proportionally to their stake and provided the same stake-management API as the stake pool owner.
 *
 * The main accounting logic in the delegation pool contract handles the following:
 * 1. Tracks how much stake each delegator owns, privately deposited as well as earned.
 * Accounting individual delegator stakes is achieved through the shares-based pool defined at
 * `aptos_std::pool_u64`, hence delegators own shares rather than absolute stakes into the delegation pool.
 * 2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime
 * and distribute them accordingly.
 * 3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)
 * from pending_inactive stake (earning rewards) and allow its delegators to withdraw the former.
 * 4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before
 * distributing them to the internal pool_u64 pools.
 *
 * In order to distinguish between stakes in different states and route rewards accordingly,
 * separate pool_u64 pools are used for individual stake states:
 *      1. one of `active` + `pending_active` stake
 *      2. one of `inactive` stake FOR each past observed lockup cycle (OLC) on the stake pool
 *      3. one of `pending_inactive` stake scheduled during this ongoing OLC
 *
 * As stake-state transitions and rewards are computed only at the stake pool level, the delegation pool
 * gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization
 * to the underlying stake pool is executed before the requested operation itself.
 *
 * At synchronization:
 *  - stake deviations between the two pools are actually the rewards produced in the meantime.
 *  - the commission fee is extracted from the rewards, the remaining stake is distributed to the internal
 * pool_u64 pools and then the commission stake used to buy shares for operator.
 *  - if detecting that the lockup expired on the stake pool, the delegation pool will isolate its
 * pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake
 * scheduled this newly started lockup.
 * Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.
 *
 * Accounting main invariants:
 *  - each stake-management operation (add/unlock/reactivate/withdraw) and operator change triggers
 * the synchronization process before executing its own function.
 *  - each OLC maps to one or more real lockups on the stake pool, but not the opposite. Actually, only a real
 * lockup with 'activity' (which inactivated some unlocking stake) triggers the creation of a new OLC.
 *  - unlocking and/or unlocked stake originating from different real lockups are never mixed together into
 * the same pool_u64. This invalidates the accounting of which rewards belong to whom.
 *  - no delegator can have unlocking and/or unlocked stake (pending withdrawals) in different OLCs. This ensures
 * delegators do not have to keep track of the OLCs when they unlocked. When creating a new pending withdrawal,
 * the existing one is executed (withdrawn) if is already inactive.
 *  - `add_stake` fees are always refunded, but only after the epoch when they have been charged ends.
 *  - withdrawing pending_inactive stake (when validator had gone inactive before its lockup expired)
 * does not inactivate any stake additional to the requested one to ensure OLC would not advance indefinitely.
 *  - the pending withdrawal exists at an OLC iff delegator owns some shares within the shares pool of that OLC.
 *
 * Example flow:
 * 1. A node operator creates a delegation pool by calling `initialize_delegation_pool` and sets
 * its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by
 * a resource account controlled by the delegation pool.
 * 2. Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64
 * 3. Operator joins the validator set as the stake pool has now the minimum stake
 * 4. The stake pool earned rewards and now has 200 active stake. A's active shares are worth 200 coins as
 * the commission fee is 0%.
 * 5a. A requests `unlock` for 100 stake
 * 5b. Synchronization detects 200 - 100 active rewards which are entirely (0% commission) added to the active pool.
 * 5c. 100 coins = (100 * 100) / 200 = 50 shares are redeemed from the active pool and exchanged for 100 shares
 * into the pending_inactive one on A's behalf
 * 6. Delegator B adds 200 stake which is converted to (200 * 50) / 100 = 100 shares into the active pool
 * 7. The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.
 * 8a. A requests `reactivate_stake` for 100 stake
 * 8b. Synchronization detects 600 - 300 active and 200 - 100 pending_inactive rewards which are both entirely
 * distributed to their corresponding pools
 * 8c. 100 coins = (100 * 100) / 200 = 50 shares are redeemed from the pending_inactive pool and exchanged for
 * (100 * 150) / 600 = 25 shares into the active one on A's behalf
 * 9. The lockup expires on the stake pool, inactivating the entire pending_inactive stake
 * 10a. B requests `unlock` for 100 stake
 * 10b. Synchronization detects no active or pending_inactive rewards, but 0 -> 100 inactive stake
 * on the stake pool, so it advances the observed lockup cycle and creates a pool_u64 for the new lockup,
 * hence allowing previous pending_inactive shares to be redeemed
 * 10c. 100 coins = (100 * 175) / 700 = 25 shares are redeemed from the active pool and exchanged for
 * 100 shares into the new pending_inactive one on B's behalf
 * 11. The stake pool earned rewards and now has some pending_inactive rewards.
 * 12a. A requests `withdraw` for its entire inactive stake
 * 12b. Synchronization detects no new inactive stake, but some pending_inactive rewards which are
 * distributed to the (2nd) pending_inactive pool
 * 12c. A's 50 shares = (50 * 100) / 50 = 100 coins are redeemed from the (1st) inactive pool and 100 stake
 * is transferred to A
 */
module aptos_framework::delegation_pool {
    use std::error;
    use std::features;
    use std::signer;
    use std::vector;

    use aptos_std::pool_u64_unbound::{Self as pool_u64, total_coins};
    use aptos_std::table::{Self, Table};

    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::stake;
    use aptos_framework::staking_config;
    use aptos_framework::timestamp;

    const MODULE_SALT: vector<u8> = b"aptos_framework::delegation_pool";

    /// Delegation pool owner capability does not exist at the provided account.
    const EOWNER_CAP_NOT_FOUND: u64 = 1;

    /// Account is already owning a delegation pool.
    const EOWNER_CAP_ALREADY_EXISTS: u64 = 2;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 3;

    /// There is a pending withdrawal to be executed before `unlock`ing any new stake.
    const EPENDING_WITHDRAWAL_EXISTS: u64 = 4;

    /// Commission percentage has to be between 0 and `MAX_FEE` - 100%.
    const EINVALID_COMMISSION_PERCENTAGE: u64 = 5;

    /// There is not enough `active` stake on the stake pool to `unlock`.
    const ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK: u64 = 6;

    /// Slashing (if implemented) should not be applied to already `inactive` stake.
    /// Not only it invalidates the accounting of past observed lockup cycles (OLC),
    /// but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
    /// Additionally, the inactive stake does not count on the voting power of validator.
    const ESLASHED_INACTIVE_STAKE_ON_PAST_OLC: u64 = 7;

    /// Delegator's active balance cannot be less than `MIN_COINS_ON_SHARES_POOL`.
    const EDELEGATOR_ACTIVE_BALANCE_TOO_LOW: u64 = 8;

    /// Delegator's pending_inactive balance cannot be less than `MIN_COINS_ON_SHARES_POOL`.
    const EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW: u64 = 9;

    /// Creating delegation pools is not enabled yet.
    const EDELEGATION_POOLS_DISABLED: u64 = 10;

    /// Cannot request to withdraw zero stake.
    const EWITHDRAW_ZERO_STAKE: u64 = 11;

    const MAX_U64: u64 = 18446744073709551615;

    /// Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285
    const MAX_FEE: u64 = 10000;

    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Special shareholder temporarily owning the `add_stake` fees charged during this epoch.
    /// On each `add_stake` operation any resulted fee is used to buy active shares for this shareholder.
    /// First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.
    const NULL_SHAREHOLDER: address = @0x0;

    /// Minimum coins to exist on a shares pool at all times.
    /// Enforced per delegator for both active and pending_inactive pools.
    /// This constraint ensures the share price cannot overly increase and lead to
    /// substantial loses when buying shares (can lose at most 1 share which may
    /// be worth a lot if current share price is high).
    /// This constraint is not enforced on inactive pools as they only allow redeems
    /// (can lose at most 1 coin regardless of current share price).
    const MIN_COINS_ON_SHARES_POOL: u64 = 1000000000;

    /// Scaling factor of shares pools used within the delegation pool
    const SHARES_SCALING_FACTOR: u64 = 10000000000000000;

    /// Capability that represents ownership over privileged operations on the underlying stake pool.
    struct DelegationPoolOwnership has key, store {
        /// equal to address of the resource account owning the stake pool
        pool_address: address,
    }

    struct ObservedLockupCycle has copy, drop, store {
        index: u64,
    }

    struct DelegationPool has key {
        // Shares pool of `active` + `pending_active` stake
        active_shares: pool_u64::Pool,
        // Index of current observed lockup cycle on the delegation pool since its creation
        observed_lockup_cycle: ObservedLockupCycle,
        // Shares pools of `inactive` stake on each ended OLC and `pending_inactive` stake on the current one.
        // Tracks shares of delegators who requested withdrawals in each OLC
        inactive_shares: Table<ObservedLockupCycle, pool_u64::Pool>,
        // Mapping from delegator address to the OLC of its pending withdrawal if having one
        pending_withdrawals: Table<address, ObservedLockupCycle>,
        // Signer capability of the resource account owning the stake pool
        stake_pool_signer_cap: account::SignerCapability,
        // Total (inactive) coins on the shares pools over all ended OLCs
        total_coins_inactive: u64,
        // Commission fee paid to the node operator out of pool rewards
        operator_commission_percentage: u64,

        // The events emitted by stake-management operations on the delegation pool
        add_stake_events: EventHandle<AddStakeEvent>,
        reactivate_stake_events: EventHandle<ReactivateStakeEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: EventHandle<WithdrawStakeEvent>,
        distribute_commission_events: EventHandle<DistributeCommissionEvent>,
    }

    struct AddStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
        add_stake_fee: u64,
    }

    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_reactivated: u64,
    }

    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64,
    }

    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64,
    }

    struct DistributeCommissionEvent has drop, store {
        pool_address: address,
        operator: address,
        commission_active: u64,
        commission_pending_inactive: u64,
    }

    #[view]
    /// Return whether supplied address `addr` is owner of a delegation pool.
    public fun owner_cap_exists(addr: address): bool {
        exists<DelegationPoolOwnership>(addr)
    }

    #[view]
    /// Return address of the delegation pool owned by `owner` or fail if there is none.
    public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership {
        assert_owner_cap_exists(owner);
        borrow_global<DelegationPoolOwnership>(owner).pool_address
    }

    #[view]
    /// Return whether a delegation pool exists at supplied address `addr`.
    public fun delegation_pool_exists(addr: address): bool {
        exists<DelegationPool>(addr)
    }

    #[view]
    /// Return the index of current observed lockup cycle on delegation pool `pool_address`.
    public fun observed_lockup_cycle(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        borrow_global<DelegationPool>(pool_address).observed_lockup_cycle.index
    }

    #[view]
    /// Return the operator commission percentage set on the delegation pool `pool_address`.
    public fun operator_commission_percentage(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        borrow_global<DelegationPool>(pool_address).operator_commission_percentage
    }

    #[view]
    /// Return the number of delegators owning active stake within `pool_address`.
    public fun shareholders_count_active_pool(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        pool_u64::shareholders_count(&borrow_global<DelegationPool>(pool_address).active_shares)
    }

    #[view]
    /// Return the stake amounts on `pool_address` in the different states:
    /// (`active`,`inactive`,`pending_active`,`pending_inactive`)
    public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) {
        assert_delegation_pool_exists(pool_address);
        stake::get_stake(pool_address)
    }

    #[view]
    /// Return whether the given delegator has any withdrawable stake. If they recently requested to unlock
    /// some stake and the stake pool's lockup cycle has not ended, their coins are not withdrawable yet.
    public fun get_pending_withdrawal(
        pool_address: address,
        delegator_address: address
    ): (bool, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            _,
            pending_inactive,
            _,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        let (withdrawal_exists, withdrawal_olc) = pending_withdrawal_exists(pool, delegator_address);
        if (!withdrawal_exists) {
            // if no pending withdrawal, there is neither inactive nor pending_inactive stake
            (false, 0)
        } else {
            // delegator has either inactive or pending_inactive stake due to automatic withdrawals
            let inactive_shares = table::borrow(&pool.inactive_shares, withdrawal_olc);
            if (withdrawal_olc.index < pool.observed_lockup_cycle.index) {
                // if withdrawal's lockup cycle ended on delegation pool then it is inactive
                (true, pool_u64::balance(inactive_shares, delegator_address))
            } else {
                pending_inactive = pool_u64::shares_to_amount_with_total_coins(
                    inactive_shares,
                    pool_u64::shares(inactive_shares, delegator_address),
                    // exclude operator pending_inactive rewards not converted to shares yet
                    pending_inactive - commission_pending_inactive
                );
                // if withdrawal's lockup cycle ended ONLY on stake pool then it is also inactive
                (lockup_cycle_ended, pending_inactive)
            }
        }
    }

    #[view]
    /// Return total stake owned by `delegator_address` within delegation pool `pool_address`
    /// in each of its individual states: (`active`,`inactive`,`pending_inactive`)
    public fun get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            active,
            _,
            commission_active,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        let total_active_shares = pool_u64::total_shares(&pool.active_shares);
        let delegator_active_shares = pool_u64::shares(&pool.active_shares, delegator_address);

        let (_, _, pending_active, _) = stake::get_stake(pool_address);
        if (pending_active == 0) {
            // zero `pending_active` stake indicates that either there are no `add_stake` fees or
            // previous epoch has ended and should identify shares owning these fees as released
            total_active_shares = total_active_shares - pool_u64::shares(&pool.active_shares, NULL_SHAREHOLDER);
            if (delegator_address == NULL_SHAREHOLDER) {
                delegator_active_shares = 0
            }
        };
        active = pool_u64::shares_to_amount_with_total_stats(
            &pool.active_shares,
            delegator_active_shares,
            // exclude operator active rewards not converted to shares yet
            active - commission_active,
            total_active_shares
        );

        // get state and stake (0 if there is none) of the pending withdrawal
        let (withdrawal_inactive, withdrawal_stake) = get_pending_withdrawal(pool_address, delegator_address);
        // report non-active stakes accordingly to the state of the pending withdrawal
        let (inactive, pending_inactive) = if (withdrawal_inactive) (withdrawal_stake, 0) else (0, withdrawal_stake);

        // should also include commission rewards in case of the operator account
        // operator rewards are actually used to buy shares which is introducing
        // some imprecision (received stake would be slightly less)
        // but adding rewards onto the existing stake is still a good approximation
        if (delegator_address == stake::get_operator(pool_address)) {
            active = active + commission_active;
            // in-flight pending_inactive commission can coexist with already inactive withdrawal
            if (lockup_cycle_ended) {
                inactive = inactive + commission_pending_inactive
            } else {
                pending_inactive = pending_inactive + commission_pending_inactive
            }
        };

        (active, inactive, pending_inactive)
    }

    #[view]
    /// Return refundable stake to be extracted from added `amount` at `add_stake` operation on pool `pool_address`.
    /// If the validator produces rewards this epoch, added stake goes directly to `pending_active` and
    /// does not earn rewards. However, all shares within a pool appreciate uniformly and when this epoch ends:
    /// - either added shares are still `pending_active` and steal from rewards of existing `active` stake
    /// - or have moved to `pending_inactive` and get full rewards (they displaced `active` stake at `unlock`)
    /// To mitigate this, some of the added stake is extracted and fed back into the pool as placeholder
    /// for the rewards the remaining stake would have earned if active:
    /// extracted-fee = (amount - extracted-fee) * reward-rate% * (100% - operator-commission%)
    public fun get_add_stake_fee(pool_address: address, amount: u64): u64 acquires DelegationPool {
        if (stake::is_current_epoch_validator(pool_address)) {
            let (rewards_rate, rewards_rate_denominator) = staking_config::get_reward_rate(&staking_config::get());
            if (rewards_rate_denominator > 0) {
                assert_delegation_pool_exists(pool_address);
                let pool = borrow_global<DelegationPool>(pool_address);

                rewards_rate = rewards_rate * (MAX_FEE - pool.operator_commission_percentage);
                rewards_rate_denominator = rewards_rate_denominator * MAX_FEE;
                ((((amount as u128) * (rewards_rate as u128)) / ((rewards_rate as u128) + (rewards_rate_denominator as u128))) as u64)
            } else { 0 }
        } else { 0 }
    }

    #[view]
    /// Return whether `pending_inactive` stake can be directly withdrawn from
    /// the delegation pool, implicitly its stake pool, in the special case
    /// the validator had gone inactive before its lockup expired.
    public fun can_withdraw_pending_inactive(pool_address: address): bool {
        stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE &&
            timestamp::now_seconds() >= stake::get_lockup_secs(pool_address)
    }

    /// Initialize a delegation pool of custom fixed `operator_commission_percentage`.
    /// A resource account is created from `owner` signer and its supplied `delegation_pool_creation_seed`
    /// to host the delegation pool resource and own the underlying stake pool.
    /// Ownership over setting the operator/voter is granted to `owner` who has both roles initially.
    public entry fun initialize_delegation_pool(
        owner: &signer,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
    ) {
        assert!(features::delegation_pools_enabled(), error::invalid_state(EDELEGATION_POOLS_DISABLED));
        let owner_address = signer::address_of(owner);
        assert!(!owner_cap_exists(owner_address), error::already_exists(EOWNER_CAP_ALREADY_EXISTS));
        assert!(operator_commission_percentage <= MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));

        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = vector::empty<u8>();
        // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts
        vector::append(&mut seed, MODULE_SALT);
        // include an additional salt in case the same resource account has already been created
        vector::append(&mut seed, delegation_pool_creation_seed);

        let (stake_pool_signer, stake_pool_signer_cap) = account::create_resource_account(owner, seed);
        coin::register<AptosCoin>(&stake_pool_signer);

        // stake_pool_signer will be owner of the stake pool and have its `stake::OwnerCapability`
        let pool_address = signer::address_of(&stake_pool_signer);
        stake::initialize_stake_owner(&stake_pool_signer, 0, owner_address, owner_address);

        let inactive_shares = table::new<ObservedLockupCycle, pool_u64::Pool>();
        table::add(
            &mut inactive_shares,
            olc_with_index(0),
            pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
        );

        move_to(&stake_pool_signer, DelegationPool {
            active_shares: pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR),
            observed_lockup_cycle: olc_with_index(0),
            inactive_shares,
            pending_withdrawals: table::new<address, ObservedLockupCycle>(),
            stake_pool_signer_cap,
            total_coins_inactive: 0,
            operator_commission_percentage,
            add_stake_events: account::new_event_handle<AddStakeEvent>(&stake_pool_signer),
            reactivate_stake_events: account::new_event_handle<ReactivateStakeEvent>(&stake_pool_signer),
            unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(&stake_pool_signer),
            withdraw_stake_events: account::new_event_handle<WithdrawStakeEvent>(&stake_pool_signer),
            distribute_commission_events: account::new_event_handle<DistributeCommissionEvent>(&stake_pool_signer),
        });

        // save delegation pool ownership and resource account address (inner stake pool address) on `owner`
        move_to(owner, DelegationPoolOwnership { pool_address });
    }

    fun assert_owner_cap_exists(owner: address) {
        assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
    }

    fun assert_delegation_pool_exists(pool_address: address) {
        assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));
    }

    fun assert_min_active_balance(pool: &DelegationPool, delegator_address: address) {
        let balance = pool_u64::balance(&pool.active_shares, delegator_address);
        assert!(balance >= MIN_COINS_ON_SHARES_POOL, error::invalid_argument(EDELEGATOR_ACTIVE_BALANCE_TOO_LOW));
    }

    fun assert_min_pending_inactive_balance(pool: &DelegationPool, delegator_address: address) {
        let balance = pool_u64::balance(pending_inactive_shares_pool(pool), delegator_address);
        assert!(
            balance >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW)
        );
    }

    fun coins_to_redeem_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool,
        shareholder: address,
        amount: u64,
    ): u64 {
        // find how many coins would be redeemed if supplying `amount`
        let redeemed_coins = pool_u64::shares_to_amount(
            src_shares_pool,
            amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
        );
        // if balance drops under threshold then redeem it entirely
        let src_balance = pool_u64::balance(src_shares_pool, shareholder);
        if (src_balance - redeemed_coins < MIN_COINS_ON_SHARES_POOL) {
            amount = src_balance;
        };
        amount
    }

    fun coins_to_transfer_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool,
        dst_shares_pool: &pool_u64::Pool,
        shareholder: address,
        amount: u64,
    ): u64 {
        // find how many coins would be redeemed from source if supplying `amount`
        let redeemed_coins = pool_u64::shares_to_amount(
            src_shares_pool,
            amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
        );
        // if balance on destination would be less than threshold then redeem difference to threshold
        let dst_balance = pool_u64::balance(dst_shares_pool, shareholder);
        if (dst_balance + redeemed_coins < MIN_COINS_ON_SHARES_POOL) {
            // `redeemed_coins` >= `amount` - 1 as redeem can lose at most 1 coin
            amount = MIN_COINS_ON_SHARES_POOL - dst_balance + 1;
        };
        // check if new `amount` drops balance on source under threshold and adjust
        coins_to_redeem_to_ensure_min_stake(src_shares_pool, shareholder, amount)
    }

    /// Retrieves the shared resource account owning the stake pool in order
    /// to forward a stake-management operation to this underlying pool.
    fun retrieve_stake_pool_owner(pool: &DelegationPool): signer {
        account::create_signer_with_capability(&pool.stake_pool_signer_cap)
    }

    /// Get the address of delegation pool reference `pool`.
    fun get_pool_address(pool: &DelegationPool): address {
        account::get_signer_capability_address(&pool.stake_pool_signer_cap)
    }

    fun olc_with_index(index: u64): ObservedLockupCycle {
        ObservedLockupCycle { index }
    }

    /// Allows an owner to change the operator of the underlying stake pool.
    public entry fun set_operator(
        owner: &signer,
        new_operator: address
    ) acquires DelegationPoolOwnership, DelegationPool {
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        // ensure the old operator is paid its uncommitted commission rewards
        synchronize_delegation_pool(pool_address);
        stake::set_operator(&retrieve_stake_pool_owner(borrow_global<DelegationPool>(pool_address)), new_operator);
    }

    /// Allows an owner to change the delegated voter of the underlying stake pool.
    public entry fun set_delegated_voter(
        owner: &signer,
        new_voter: address
    ) acquires DelegationPoolOwnership, DelegationPool {
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        stake::set_delegated_voter(&retrieve_stake_pool_owner(borrow_global<DelegationPool>(pool_address)), new_voter);
    }

    /// Add `amount` of coins to the delegation pool `pool_address`.
    public entry fun add_stake(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // short-circuit if amount to add is 0 so no event is emitted
        if (amount == 0) { return };
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        // fee to be charged for adding `amount` stake on this delegation pool at this epoch
        let add_stake_fee = get_add_stake_fee(pool_address, amount);

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        // stake the entire amount to the stake pool
        coin::transfer<AptosCoin>(delegator, pool_address, amount);
        stake::add_stake(&retrieve_stake_pool_owner(pool), amount);

        // but buy shares for delegator just for the remaining amount after fee
        pool_u64::buy_in(&mut pool.active_shares, delegator_address, amount - add_stake_fee);
        assert_min_active_balance(pool, delegator_address);

        // grant temporary ownership over `add_stake` fees to a separate shareholder in order to:
        // - not mistake them for rewards to pay the operator from
        // - distribute them together with the `active` rewards when this epoch ends
        // in order to appreciate all shares on the active pool atomically
        pool_u64::buy_in(&mut pool.active_shares, NULL_SHAREHOLDER, add_stake_fee);

        event::emit_event(
            &mut pool.add_stake_events,
            AddStakeEvent {
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee,
            },
        );
    }

    /// Unlock `amount` from the active + pending_active stake of `delegator` or
    /// at most how much active stake there is on the stake pool.
    public entry fun unlock(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // short-circuit if amount to unlock is 0 so no event is emitted
        if (amount == 0) { return };

        // fail unlock of more stake than `active` on the stake pool
        let (active, _, _, _) = stake::get_stake(pool_address);
        assert!(amount <= active, error::invalid_argument(ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK));

        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        amount = coins_to_transfer_to_ensure_min_stake(
            &pool.active_shares,
            pending_inactive_shares_pool(pool),
            delegator_address,
            amount,
        );
        amount = redeem_active_shares(pool, delegator_address, amount);

        stake::unlock(&retrieve_stake_pool_owner(pool), amount);

        buy_in_pending_inactive_shares(pool, delegator_address, amount);
        assert_min_pending_inactive_balance(pool, delegator_address);

        event::emit_event(
            &mut pool.unlock_stake_events,
            UnlockStakeEvent {
                pool_address,
                delegator_address,
                amount_unlocked: amount,
            },
        );
    }

    /// Move `amount` of coins from pending_inactive to active.
    public entry fun reactivate_stake(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // short-circuit if amount to reactivate is 0 so no event is emitted
        if (amount == 0) { return };
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        amount = coins_to_transfer_to_ensure_min_stake(
            pending_inactive_shares_pool(pool),
            &pool.active_shares,
            delegator_address,
            amount,
        );
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        amount = redeem_inactive_shares(pool, delegator_address, amount, observed_lockup_cycle);

        stake::reactivate_stake(&retrieve_stake_pool_owner(pool), amount);

        pool_u64::buy_in(&mut pool.active_shares, delegator_address, amount);
        assert_min_active_balance(pool, delegator_address);

        event::emit_event(
            &mut pool.reactivate_stake_events,
            ReactivateStakeEvent {
                pool_address,
                delegator_address,
                amount_reactivated: amount,
            },
        );
    }

    /// Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address`.
    public entry fun withdraw(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        assert!(amount > 0, error::invalid_argument(EWITHDRAW_ZERO_STAKE));
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        withdraw_internal(borrow_global_mut<DelegationPool>(pool_address), signer::address_of(delegator), amount);
    }

    fun withdraw_internal(pool: &mut DelegationPool, delegator_address: address, amount: u64) {
        // short-circuit if amount to withdraw is 0 so no event is emitted
        if (amount == 0) { return };

        let pool_address = get_pool_address(pool);
        let (withdrawal_exists, withdrawal_olc) = pending_withdrawal_exists(pool, delegator_address);
        // exit if no withdrawal or (it is pending and cannot withdraw pending_inactive stake from stake pool)
        if (!(
            withdrawal_exists &&
                (withdrawal_olc.index < pool.observed_lockup_cycle.index || can_withdraw_pending_inactive(pool_address))
        )) { return };

        if (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
            amount = coins_to_redeem_to_ensure_min_stake(
                pending_inactive_shares_pool(pool),
                delegator_address,
                amount,
            )
        };
        amount = redeem_inactive_shares(pool, delegator_address, amount, withdrawal_olc);

        let stake_pool_owner = &retrieve_stake_pool_owner(pool);
        // stake pool will inactivate entire pending_inactive stake at `stake::withdraw` to make it withdrawable
        // however, bypassing the inactivation of excess stake (inactivated but not withdrawn) ensures
        // the OLC is not advanced indefinitely on `unlock`-`withdraw` paired calls
        if (can_withdraw_pending_inactive(pool_address)) {
            // get excess stake before being entirely inactivated
            let (_, _, _, pending_inactive) = stake::get_stake(pool_address);
            if (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
                // `amount` less excess if withdrawing pending_inactive stake
                pending_inactive = pending_inactive - amount
            };
            // escape excess stake from inactivation
            stake::reactivate_stake(stake_pool_owner, pending_inactive);
            stake::withdraw(stake_pool_owner, amount);
            // restore excess stake to the pending_inactive state
            stake::unlock(stake_pool_owner, pending_inactive);
        } else {
            // no excess stake if `stake::withdraw` does not inactivate at all
            stake::withdraw(stake_pool_owner, amount);
        };
        coin::transfer<AptosCoin>(stake_pool_owner, delegator_address, amount);

        // commit withdrawal of possibly inactive stake to the `total_coins_inactive`
        // known by the delegation pool in order to not mistake it for slashing at next synchronization
        let (_, inactive, _, _) = stake::get_stake(pool_address);
        pool.total_coins_inactive = inactive;

        event::emit_event(
            &mut pool.withdraw_stake_events,
            WithdrawStakeEvent {
                pool_address,
                delegator_address,
                amount_withdrawn: amount,
            },
        );
    }

    /// Return the unique observed lockup cycle where delegator `delegator_address` may have
    /// unlocking (or already unlocked) stake to be withdrawn from delegation pool `pool`.
    /// A bool is returned to signal if a pending withdrawal exists at all.
    fun pending_withdrawal_exists(pool: &DelegationPool, delegator_address: address): (bool, ObservedLockupCycle) {
        if (table::contains(&pool.pending_withdrawals, delegator_address)) {
            (true, *table::borrow(&pool.pending_withdrawals, delegator_address))
        } else {
            (false, olc_with_index(0))
        }
    }

    /// Return a mutable reference to the shares pool of `pending_inactive` stake on the
    /// delegation pool, always the last item in `inactive_shares`.
    fun pending_inactive_shares_pool_mut(pool: &mut DelegationPool): &mut pool_u64::Pool {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        table::borrow_mut(&mut pool.inactive_shares, observed_lockup_cycle)
    }

    fun pending_inactive_shares_pool(pool: &DelegationPool): &pool_u64::Pool {
        table::borrow(&pool.inactive_shares, pool.observed_lockup_cycle)
    }

    /// Execute the pending withdrawal of `delegator_address` on delegation pool `pool`
    /// if existing and already inactive to allow the creation of a new one.
    /// `pending_inactive` stake would be left untouched even if withdrawable and should
    /// be explicitly withdrawn by delegator
    fun execute_pending_withdrawal(pool: &mut DelegationPool, delegator_address: address) {
        let (withdrawal_exists, withdrawal_olc) = pending_withdrawal_exists(pool, delegator_address);
        if (withdrawal_exists && withdrawal_olc.index < pool.observed_lockup_cycle.index) {
            withdraw_internal(pool, delegator_address, MAX_U64);
        }
    }

    /// Buy shares into the pending_inactive pool on behalf of delegator `shareholder` who
    /// redeemed `coins_amount` from the active pool to schedule it for unlocking.
    /// If delegator's pending withdrawal exists and has been inactivated, execute it firstly
    /// to ensure there is always only one withdrawal request.
    fun buy_in_pending_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        // cannot buy inactive shares, only pending_inactive at current lockup cycle
        let new_shares = pool_u64::buy_in(pending_inactive_shares_pool_mut(pool), shareholder, coins_amount);
        // never create a new pending withdrawal unless delegator owns some pending_inactive shares
        if (new_shares == 0) { return 0 };

        // execute the pending withdrawal if exists and is inactive before creating a new one
        execute_pending_withdrawal(pool, shareholder);

        // save observed lockup cycle for the new pending withdrawal
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        assert!(*table::borrow_mut_with_default(
            &mut pool.pending_withdrawals,
            shareholder,
            observed_lockup_cycle
        ) == observed_lockup_cycle,
            error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)
        );

        new_shares
    }

    /// Convert `coins_amount` of coins to be redeemed from shares pool `shares_pool`
    /// to the exact number of shares to redeem in order to achieve this.
    fun amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u128 {
        if (coins_amount >= pool_u64::balance(shares_pool, shareholder)) {
            // cap result at total shares of shareholder to pass `EINSUFFICIENT_SHARES` on subsequent redeem
            pool_u64::shares(shares_pool, shareholder)
        } else {
            pool_u64::amount_to_shares(shares_pool, coins_amount)
        }
    }

    /// Redeem shares from the active pool on behalf of delegator `shareholder` who
    /// wants to unlock `coins_amount` of its active stake.
    /// Extracted coins will be used to buy shares into the pending_inactive pool and
    /// be available for withdrawal when current OLC ends.
    fun redeem_active_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        let shares_to_redeem = amount_to_shares_to_redeem(&pool.active_shares, shareholder, coins_amount);
        // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
        if (shares_to_redeem == 0) return 0;
        pool_u64::redeem_shares(&mut pool.active_shares, shareholder, shares_to_redeem)
    }

    /// Redeem shares from the inactive pool at `lockup_cycle` < current OLC on behalf of
    /// delegator `shareholder` who wants to withdraw `coins_amount` of its unlocked stake.
    /// Redeem shares from the pending_inactive pool at `lockup_cycle` == current OLC on behalf of
    /// delegator `shareholder` who wants to reactivate `coins_amount` of its unlocking stake.
    /// For latter case, extracted coins will be used to buy shares into the active pool and
    /// escape inactivation when current lockup ends.
    fun redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: ObservedLockupCycle,
    ): u64 {
        let inactive_shares = table::borrow_mut(&mut pool.inactive_shares, lockup_cycle);
        let shares_to_redeem = amount_to_shares_to_redeem(inactive_shares, shareholder, coins_amount);
        // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
        if (shares_to_redeem == 0) return 0;
        // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`
        let redeemed_coins = pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

        // if entirely reactivated pending_inactive stake or withdrawn inactive one,
        // re-enable unlocking for delegator by deleting this pending withdrawal
        if (pool_u64::shares(inactive_shares, shareholder) == 0) {
            // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal
            // 1 & 2: the pending withdrawal itself has been emptied of shares and can be safely deleted
            table::remove(&mut pool.pending_withdrawals, shareholder);
        };
        // destroy inactive shares pool of past OLC if all its stake has been withdrawn
        if (lockup_cycle.index < pool.observed_lockup_cycle.index && total_coins(inactive_shares) == 0) {
            pool_u64::destroy_empty(table::remove(&mut pool.inactive_shares, lockup_cycle));
        };

        redeemed_coins
    }

    /// Calculate stake deviations between the delegation and stake pools in order to
    /// capture the rewards earned in the meantime, resulted operator commission and
    /// whether the lockup expired on the stake pool.
    fun calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        let (active, inactive, pending_active, pending_inactive) = stake::get_stake(get_pool_address(pool));
        assert!(
            inactive >= pool.total_coins_inactive,
            error::invalid_state(ESLASHED_INACTIVE_STAKE_ON_PAST_OLC)
        );
        // determine whether a new lockup cycle has been ended on the stake pool and
        // inactivated SOME `pending_inactive` stake which should stop earning rewards now,
        // thus requiring separation of the `pending_inactive` stake on current observed lockup
        // and the future one on the newly started lockup
        let lockup_cycle_ended = inactive > pool.total_coins_inactive;

        // actual coins on stake pool belonging to the active shares pool
        active = active + pending_active;
        // actual coins on stake pool belonging to the shares pool hosting `pending_inactive` stake
        // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:
        if (lockup_cycle_ended) {
            // `inactive` on stake pool = any previous `inactive` stake +
            // any previous `pending_inactive` stake and its rewards (both inactivated)
            pending_inactive = inactive - pool.total_coins_inactive
        };

        // on stake-management operations, total coins on the internal shares pools and individual
        // stakes on the stake pool are updated simultaneously, thus the only stakes becoming
        // unsynced are rewards and slashes routed exclusively to/out the stake pool

        // operator `active` rewards not persisted yet to the active shares pool
        let commission_active = total_coins(&pool.active_shares);
        commission_active = if (active > commission_active) {
            multiply_then_divide(active - commission_active, pool.operator_commission_percentage, MAX_FEE)
        } else {
            // handle any slashing applied to `active` stake
            0
        };
        // operator `pending_inactive` rewards not persisted yet to the pending_inactive shares pool
        let commission_pending_inactive = total_coins(pending_inactive_shares_pool(pool));
        commission_pending_inactive = if (pending_inactive > commission_pending_inactive) {
            multiply_then_divide(
                pending_inactive - commission_pending_inactive,
                pool.operator_commission_percentage,
                MAX_FEE
            )
        } else {
            // handle any slashing applied to `pending_inactive` stake
            0
        };

        (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)
    }

    /// Synchronize delegation and stake pools: distribute yet-undetected rewards to the corresponding internal
    /// shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.
    public entry fun synchronize_delegation_pool(pool_address: address) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            active,
            pending_inactive,
            commission_active,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        // zero `pending_active` stake indicates that either there are no `add_stake` fees or
        // previous epoch has ended and should release the shares owning the existing fees
        let (_, _, pending_active, _) = stake::get_stake(pool_address);
        if (pending_active == 0) {
            // renounce ownership over the `add_stake` fees by redeeming all shares of
            // the special shareholder, implicitly their equivalent coins, out of the active shares pool
            redeem_active_shares(pool, NULL_SHAREHOLDER, MAX_U64);
        };

        // distribute rewards remaining after commission, to delegators (to already existing shares)
        // before buying shares for the operator for its entire commission fee
        // otherwise, operator's new shares would additionally appreciate from rewards it does not own

        // update total coins accumulated by `active` + `pending_active` shares
        // redeemed `add_stake` fees are restored and distributed to the rest of the pool as rewards
        pool_u64::update_total_coins(&mut pool.active_shares, active - commission_active);
        // update total coins accumulated by `pending_inactive` shares at current observed lockup cycle
        pool_u64::update_total_coins(
            pending_inactive_shares_pool_mut(pool),
            pending_inactive - commission_pending_inactive
        );

        // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)
        pool_u64::buy_in(&mut pool.active_shares, stake::get_operator(pool_address), commission_active);
        // reward operator its commission out of uncommitted pending_inactive rewards
        buy_in_pending_inactive_shares(pool, stake::get_operator(pool_address), commission_pending_inactive);

        event::emit_event(
            &mut pool.distribute_commission_events,
            DistributeCommissionEvent {
                pool_address,
                operator: stake::get_operator(pool_address),
                commission_active,
                commission_pending_inactive,
            },
        );

        // advance lockup cycle on delegation pool if already ended on stake pool (AND stake explicitly inactivated)
        if (lockup_cycle_ended) {
            // capture inactive coins over all ended lockup cycles (including this ending one)
            let (_, inactive, _, _) = stake::get_stake(pool_address);
            pool.total_coins_inactive = inactive;

            // advance lockup cycle on the delegation pool
            pool.observed_lockup_cycle.index = pool.observed_lockup_cycle.index + 1;
            // start new lockup cycle with a fresh shares pool for `pending_inactive` stake
            table::add(
                &mut pool.inactive_shares,
                pool.observed_lockup_cycle,
                pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
            );
        }
    }

    public fun multiply_then_divide(x: u64, y: u64, z: u64): u64 {
        let result = (to_u128(x) * to_u128(y)) / to_u128(z);
        (result as u64)
    }

    fun to_u128(num: u64): u128 {
        (num as u128)
    }

    #[test_only]
    use aptos_framework::reconfiguration;
    #[test_only]
    use aptos_std::fixed_point64;
    #[test_only]
    use aptos_framework::timestamp::fast_forward_seconds;

    #[test_only]
    const CONSENSUS_KEY_1: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP_1: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    const EPOCH_DURATION: u64 = 60;
    #[test_only]
    const LOCKUP_CYCLE_SECONDS: u64 = 3600;

    #[test_only]
    const ONE_APT: u64 = 100000000;

    #[test_only]
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    #[test_only]
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    #[test_only]
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;

    #[test_only]
    const DELEGATION_POOLS: u64 = 11;

    #[test_only]
    public fun end_aptos_epoch() {
        stake::end_epoch(); // additionally forwards EPOCH_DURATION seconds
        reconfiguration::reconfigure_for_test_custom();
    }

    #[test_only]
    public fun initialize_for_test(aptos_framework: &signer) {
        initialize_for_test_custom(
            aptos_framework,
            100 * ONE_APT,
            10000 * ONE_APT,
            LOCKUP_CYCLE_SECONDS,
            true,
            1,
            100,
            1000000
        );
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
        features::change_feature_flags(aptos_framework, vector[DELEGATION_POOLS], vector[]);
    }

    #[test_only]
    public fun initialize_test_validator(
        validator: &signer,
        amount: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
    ) acquires DelegationPoolOwnership, DelegationPool {
        let validator_address = signer::address_of(validator);
        if (!account::exists_at(validator_address)) {
            account::create_account_for_test(validator_address);
        };

        initialize_delegation_pool(validator, 0, vector::empty<u8>());
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

    #[test_only]
    fun unlock_with_min_stake_disabled(
        delegator: &signer,
        pool_address: address,
        amount: u64
    ) acquires DelegationPool {
        synchronize_delegation_pool(pool_address);

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        amount = redeem_active_shares(pool, delegator_address, amount);
        stake::unlock(&retrieve_stake_pool_owner(pool), amount);
        buy_in_pending_inactive_shares(pool, delegator_address, amount);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    public entry fun test_delegation_pools_disabled(
        aptos_framework: &signer,
        validator: &signer,
    ) {
        initialize_for_test(aptos_framework);
        features::change_feature_flags(aptos_framework, vector[], vector[DELEGATION_POOLS]);

        initialize_delegation_pool(validator, 0, vector::empty<u8>());
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_set_operator_and_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, 0, vector::empty<u8>());
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
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        // account does not own any delegation pool
        set_operator(validator, @0x111);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
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
        initialize_delegation_pool(validator, 0, x"00");
        initialize_delegation_pool(validator, 0, x"01");
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x1000B, location = Self)]
    public entry fun test_cannot_withdraw_zero_stake(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_delegation_pool(validator, 0, x"00");
        withdraw(validator, get_owned_pool_address(signer::address_of(validator)), 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_initialize_delegation_pool(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, 1234, vector::empty<u8>());

        assert_owner_cap_exists(validator_address);
        let pool_address = get_owned_pool_address(validator_address);
        assert_delegation_pool_exists(pool_address);

        assert!(stake::stake_pool_exists(pool_address), 0);
        assert!(stake::get_operator(pool_address) == validator_address, 0);
        assert!(stake::get_delegated_voter(pool_address) == validator_address, 0);

        assert!(observed_lockup_cycle(pool_address) == 0, 0);
        assert!(total_coins_inactive(pool_address) == 0, 0);
        assert!(operator_commission_percentage(pool_address) == 1234, 0);
        assert_inactive_shares_pool(pool_address, 0, true, 0);
        stake::assert_stake_pool(pool_address, 0, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_add_stake_fee(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test_custom(
            aptos_framework,
            100 * ONE_APT,
            10000000 * ONE_APT,
            LOCKUP_CYCLE_SECONDS,
            true,
            1,
            100,
            1000000
        );

        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);

        // create delegation pool with 37.35% operator commission
        initialize_delegation_pool(validator, 3735, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);

        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);

        // zero `add_stake` fee as validator is not producing rewards this epoch
        assert!(get_add_stake_fee(pool_address, 1000000 * ONE_APT) == 0, 0);

        // add 1M APT, join the validator set and activate this stake
        stake::mint(validator, 1000000 * ONE_APT);
        add_stake(validator, pool_address, 1000000 * ONE_APT);

        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        // `add_stake` fee for 100000 coins: 100000 * 0.006265 / (1 + 0.006265)
        assert!(get_add_stake_fee(pool_address, 100000 * ONE_APT) == 62259941466, 0);

        // add pending_active stake from multiple delegators
        stake::mint(delegator1, 100000 * ONE_APT);
        add_stake(delegator1, pool_address, 100000 * ONE_APT);
        stake::mint(delegator2, 10000 * ONE_APT);
        add_stake(delegator2, pool_address, 10000 * ONE_APT);

        end_aptos_epoch();
        // delegators should own the same amount as initially deposited
        assert_delegation(delegator1_address, pool_address, 10000000000000, 0, 0);
        assert_delegation(delegator2_address, pool_address, 1000000000000, 0, 0);

        // add more stake from delegator 1
        stake::mint(delegator1, 10000 * ONE_APT);
        let (delegator1_active, _, _) = get_stake(pool_address, delegator1_address);
        add_stake(delegator1, pool_address, 10000 * ONE_APT);

        let fee = get_add_stake_fee(pool_address, 10000 * ONE_APT);
        assert_delegation(delegator1_address, pool_address, delegator1_active + 10000 * ONE_APT - fee, 0, 0);

        // delegator 2 should not benefit in any way from this new stake
        assert_delegation(delegator2_address, pool_address, 1000000000000, 0, 0);

        // add more stake from delegator 2
        stake::mint(delegator2, 100000 * ONE_APT);
        add_stake(delegator2, pool_address, 100000 * ONE_APT);

        end_aptos_epoch();
        // delegators should own the same amount as initially deposited + any rewards produced
        // 10000000000000 * 1% * (100 - 37.35)%
        assert_delegation(delegator1_address, pool_address, 11062650000001, 0, 0);
        // 1000000000000 * 1% * (100 - 37.35)%
        assert_delegation(delegator2_address, pool_address, 11006265000001, 0, 0);

        // in-flight operator commission rewards do not automatically restake/compound
        synchronize_delegation_pool(pool_address);

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        assert_delegation(delegator1_address, pool_address, 11062650000001, 0, 0);
        assert_delegation(delegator2_address, pool_address, 11006265000001, 0, 0);

        end_aptos_epoch();
        // delegators should own previous stake * 1.006265
        assert_delegation(delegator1_address, pool_address, 11131957502251, 0, 0);
        assert_delegation(delegator2_address, pool_address, 11075219250226, 0, 0);

        // add more stake from delegator 1
        stake::mint(delegator1, 20000 * ONE_APT);
        (delegator1_active, _, _) = get_stake(pool_address, delegator1_address);
        add_stake(delegator1, pool_address, 20000 * ONE_APT);

        fee = get_add_stake_fee(pool_address, 20000 * ONE_APT);
        assert_delegation(delegator1_address, pool_address, delegator1_active + 20000 * ONE_APT - fee, 0, 0);

        // delegator 1 unlocks his entire newly added stake
        unlock(delegator1, pool_address, 20000 * ONE_APT - fee);
        end_aptos_epoch();
        // delegator 1 should own previous 11131957502250 active * 1.006265 and 20000 coins pending_inactive
        assert_delegation(delegator1_address, pool_address, 11201699216002, 0, 2000000000000);

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        synchronize_delegation_pool(pool_address);
        assert_delegation(delegator1_address, pool_address, 11201699216002, 0, 2000000000000);

        let reward_period_start_time_in_sec = timestamp::now_seconds();
        // Enable rewards rate decrease. Initially rewards rate is still 1% every epoch. Rewards rate halves every year.
        let one_year_in_secs: u64 = 31536000;
        staking_config::initialize_rewards(
            aptos_framework,
            fixed_point64::create_from_rational(2, 100),
            fixed_point64::create_from_rational(6, 1000),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(50, 100),
        );
        features::change_feature_flags(aptos_framework, vector[features::get_periodical_reward_rate_decrease_feature()], vector[]);

        // add more stake from delegator 1
        stake::mint(delegator1, 20000 * ONE_APT);
        let delegator1_pending_inactive: u64;
        (delegator1_active, _, delegator1_pending_inactive) = get_stake(pool_address, delegator1_address);
        fee = get_add_stake_fee(pool_address, 20000 * ONE_APT);
        add_stake(delegator1, pool_address, 20000 * ONE_APT);

        assert_delegation(delegator1_address, pool_address, delegator1_active + 20000 * ONE_APT - fee, 0, delegator1_pending_inactive);

        // delegator 1 unlocks his entire newly added stake
        unlock(delegator1, pool_address, 20000 * ONE_APT - fee);
        end_aptos_epoch();
        // delegator 1 should own previous 11201699216002 active * ~1.01253 and 20000 * ~1.01253 + 20000 coins pending_inactive
        assert_delegation(delegator1_address, pool_address, 11342056366822, 0, 4025059974939);

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        synchronize_delegation_pool(pool_address);
        assert_delegation(delegator1_address, pool_address, 11342056366822, 0, 4025059974939);

        fast_forward_seconds(one_year_in_secs);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_never_create_pending_withdrawal_if_no_shares_bought(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, false);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add stake without fees as validator is not active yet
        stake::mint(delegator, 10 * ONE_APT);
        add_stake(delegator, pool_address, 10 * ONE_APT);
        end_aptos_epoch();

        unlock(validator, pool_address, 100 * ONE_APT);

        stake::assert_stake_pool(pool_address, 91000000000, 0, 0, 10000000000);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 91910000000, 0, 0, 10100000000);

        unlock_with_min_stake_disabled(delegator, pool_address, 1);
        // request 1 coins * 910 / 919.1 = 0.99 shares to redeem * 1.01 price -> 0 coins out
        // 1 coins lost at redeem due to 0.99 shares being burned
        assert_delegation(delegator_address, pool_address, 1009999999, 0, 0);
        assert_pending_withdrawal(delegator_address, pool_address, false, 0, false, 0);

        unlock_with_min_stake_disabled(delegator, pool_address, 2);
        // request 2 coins * 909.99 / 919.1 = 1.98 shares to redeem * 1.01 price -> 1 coins out
        // with 1 coins buy 1 * 100 / 101 = 0.99 shares in pending_inactive pool * 1.01 -> 0 coins in
        // 1 coins lost at redeem due to 1.98 - 1.01 shares being burned + 1 coins extracted
        synchronize_delegation_pool(pool_address);
        assert_delegation(delegator_address, pool_address, 1009999997, 0, 0);
        // the pending withdrawal has been created as > 0 pending_inactive shares have been bought
        assert_pending_withdrawal(delegator_address, pool_address, true, 0, false, 0);

        // successfully delete the pending withdrawal (redeem all owned shares even worth 0 coins)
        reactivate_stake(delegator, pool_address, 1);
        assert_delegation(delegator_address, pool_address, 1009999997, 0, 0);
        assert_pending_withdrawal(delegator_address, pool_address, false, 0, false, 0);

        // unlock min coins to own some pending_inactive balance (have to disable min-balance checks)
        unlock_with_min_stake_disabled(delegator, pool_address, 3);
        // request 3 coins * 909.99 / 919.09 = 2.97 shares to redeem * 1.01 price -> 2 coins out
        // with 2 coins buy 2 * 100 / 101 = 1.98 shares in pending_inactive pool * 1.01 -> 1 coins in
        // 1 coins lost at redeem due to 2.97 - 2 * 1.01 shares being burned + 2 coins extracted
        synchronize_delegation_pool(pool_address);
        assert_delegation(delegator_address, pool_address, 1009999994, 0, 1);
        // the pending withdrawal has been created as > 0 pending_inactive shares have been bought
        assert_pending_withdrawal(delegator_address, pool_address, true, 0, false, 1);

        reactivate_stake(delegator, pool_address, 1);
        // redeem 1 coins >= delegator balance -> all shares are redeemed and pending withdrawal is deleted
        assert_delegation(delegator_address, pool_address, 1009999995, 0, 0);
        // the pending withdrawal has been deleted as delegator has 0 pending_inactive shares now
        assert_pending_withdrawal(delegator_address, pool_address, false, 0, false, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    public entry fun test_add_stake_min_amount(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, MIN_COINS_ON_SHARES_POOL - 1, false, false);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_add_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, false, false);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // validator is inactive => added stake is `active` by default
        stake::assert_stake_pool(pool_address, 1000 * ONE_APT, 0, 0, 0);
        assert_delegation(validator_address, pool_address, 1000 * ONE_APT, 0, 0);

        // zero `add_stake` fee as validator is not producing rewards this epoch
        assert!(get_add_stake_fee(pool_address, 250 * ONE_APT) == 0, 0);

        // check `add_stake` increases `active` stakes of delegator and stake pool
        stake::mint(validator, 300 * ONE_APT);
        let balance = coin::balance<AptosCoin>(validator_address);
        add_stake(validator, pool_address, 250 * ONE_APT);

        // check added stake have been transferred out of delegator account
        assert!(coin::balance<AptosCoin>(validator_address) == balance - 250 * ONE_APT, 0);
        // zero `add_stake` fee charged from added stake
        assert_delegation(validator_address, pool_address, 1250 * ONE_APT, 0, 0);
        // zero `add_stake` fee transferred to null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        // added stake is automatically `active` on inactive validator
        stake::assert_stake_pool(pool_address, 1250 * ONE_APT, 0, 0, 0);

        // activate validator
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // add 250 coins being pending_active until next epoch
        stake::mint(validator, 250 * ONE_APT);
        add_stake(validator, pool_address, 250 * ONE_APT);

        let fee1 = get_add_stake_fee(pool_address, 250 * ONE_APT);
        assert_delegation(validator_address, pool_address, 1500 * ONE_APT - fee1, 0, 0);
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1, 0, 0);
        stake::assert_stake_pool(pool_address, 1250 * ONE_APT, 0, 250 * ONE_APT, 0);

        // add 100 additional coins being pending_active until next epoch
        stake::mint(validator, 100 * ONE_APT);
        add_stake(validator, pool_address, 100 * ONE_APT);

        let fee2 = get_add_stake_fee(pool_address, 100 * ONE_APT);
        assert_delegation(validator_address, pool_address, 1600 * ONE_APT - fee1 - fee2, 0, 0);
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1 + fee2, 0, 0);
        stake::assert_stake_pool(pool_address, 1250 * ONE_APT, 0, 350 * ONE_APT, 0);

        end_aptos_epoch();
        // delegator got its `add_stake` fees back + 1250 * 1% * (100% - 0%) active rewards
        assert_delegation(validator_address, pool_address, 161250000000, 0, 0);
        stake::assert_stake_pool(pool_address, 161250000000, 0, 0, 0);

        // check that shares of null shareholder have been released
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        synchronize_delegation_pool(pool_address);
        assert!(pool_u64::shares(&borrow_global<DelegationPool>(pool_address).active_shares, NULL_SHAREHOLDER) == 0, 0);
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);

        // add 200 coins being pending_active until next epoch
        stake::mint(validator, 200 * ONE_APT);
        add_stake(validator, pool_address, 200 * ONE_APT);

        fee1 = get_add_stake_fee(pool_address, 200 * ONE_APT);
        assert_delegation(validator_address, pool_address, 181250000000 - fee1, 0, 0);
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1 - 1, 0, 0);
        stake::assert_stake_pool(pool_address, 161250000000, 0, 20000000000, 0);

        end_aptos_epoch();
        // delegator got its `add_stake` fee back + 161250000000 * 1% active rewards
        assert_delegation(validator_address, pool_address, 182862500000, 0, 0);
        stake::assert_stake_pool(pool_address, 182862500000, 0, 0, 0);

        // check that shares of null shareholder have been released
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        synchronize_delegation_pool(pool_address);
        assert!(pool_u64::shares(&borrow_global<DelegationPool>(pool_address).active_shares, NULL_SHAREHOLDER) == 0, 0);
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_add_stake_many(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::assert_stake_pool(pool_address, 1000 * ONE_APT, 0, 0, 0);
        assert_delegation(validator_address, pool_address, 1000 * ONE_APT, 0, 0);

        // add 250 coins from second account
        stake::mint(delegator, 250 * ONE_APT);
        add_stake(delegator, pool_address, 250 * ONE_APT);

        let fee1 = get_add_stake_fee(pool_address, 250 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 250 * ONE_APT - fee1, 0, 0);
        assert_delegation(validator_address, pool_address, 1000 * ONE_APT, 0, 0);
        stake::assert_stake_pool(pool_address, 1000 * ONE_APT, 0, 250 * ONE_APT, 0);

        end_aptos_epoch();
        // 1000 * 1.01 active stake + 250 pending_active stake
        stake::assert_stake_pool(pool_address, 1260 * ONE_APT, 0, 0, 0);
        // delegator got its `add_stake` fee back
        assert_delegation(delegator_address, pool_address, 250 * ONE_APT, 0, 0);
        // actual active rewards have been distributed to their earner(s)
        assert_delegation(validator_address, pool_address, 100999999999, 0, 0);

        // add another 250 coins from first account
        stake::mint(validator, 250 * ONE_APT);
        add_stake(validator, pool_address, 250 * ONE_APT);

        fee1 = get_add_stake_fee(pool_address, 250 * ONE_APT);
        assert_delegation(validator_address, pool_address, 125999999999 - fee1, 0, 0);
        assert_delegation(delegator_address, pool_address, 250 * ONE_APT, 0, 0);
        stake::assert_stake_pool(pool_address, 1260 * ONE_APT, 0, 250 * ONE_APT, 0);

        // add another 100 coins from second account
        stake::mint(delegator, 100 * ONE_APT);
        add_stake(delegator, pool_address, 100 * ONE_APT);

        let fee2 = get_add_stake_fee(pool_address, 100 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 350 * ONE_APT - fee2, 0, 0);
        assert_delegation(validator_address, pool_address, 125999999999 - fee1, 0, 0);
        stake::assert_stake_pool(pool_address, 1260 * ONE_APT, 0, 350 * ONE_APT, 0);

        end_aptos_epoch();
        // both delegators got their `add_stake` fees back
        // 250 * 1.01 active stake + 100 pending_active stake
        assert_delegation(delegator_address, pool_address, 35250000001, 0, 0);
        // 1010 * 1.01 active stake + 250 pending_active stake
        assert_delegation(validator_address, pool_address, 127009999998, 0, 0);
        stake::assert_stake_pool(pool_address, 162260000000, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_unlock_single(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 100 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add 200 coins pending_active until next epoch
        stake::mint(validator, 200 * ONE_APT);
        add_stake(validator, pool_address, 200 * ONE_APT);

        let fee = get_add_stake_fee(pool_address, 200 * ONE_APT);
        assert_delegation(validator_address, pool_address, 300 * ONE_APT - fee, 0, 0);
        stake::assert_stake_pool(pool_address, 100 * ONE_APT, 0, 200 * ONE_APT, 0);

        // cannot unlock pending_active stake (only 100/300 stake can be displaced)
        unlock(validator, pool_address, 100 * ONE_APT);
        assert_delegation(validator_address, pool_address, 200 * ONE_APT - fee, 0, 100 * ONE_APT);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 100 * ONE_APT);
        stake::assert_stake_pool(pool_address, 0, 0, 200 * ONE_APT, 100 * ONE_APT);
        assert_inactive_shares_pool(pool_address, 0, true, 100 * ONE_APT);

        // reactivate entire pending_inactive stake progressively
        reactivate_stake(validator, pool_address, 50 * ONE_APT);

        assert_delegation(validator_address, pool_address, 250 * ONE_APT - fee, 0, 50 * ONE_APT);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 50 * ONE_APT);
        stake::assert_stake_pool(pool_address, 50 * ONE_APT, 0, 200 * ONE_APT, 50 * ONE_APT);

        reactivate_stake(validator, pool_address, 50 * ONE_APT);

        assert_delegation(validator_address, pool_address, 300 * ONE_APT - fee, 0, 0);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        stake::assert_stake_pool(pool_address, 100 * ONE_APT, 0, 200 * ONE_APT, 0);
        // pending_inactive shares pool has not been deleted (as can still `unlock` this OLC)
        assert_inactive_shares_pool(pool_address, 0, true, 0);

        end_aptos_epoch();
        // 10000000000 * 1.01 active stake + 20000000000 pending_active stake
        assert_delegation(validator_address, pool_address, 301 * ONE_APT, 0, 0);
        stake::assert_stake_pool(pool_address, 301 * ONE_APT, 0, 0, 0);

        // can unlock more than at previous epoch as the pending_active stake became active
        unlock(validator, pool_address, 150 * ONE_APT);
        assert_delegation(validator_address, pool_address, 15100000001, 0, 14999999999);
        stake::assert_stake_pool(pool_address, 15100000001, 0, 0, 14999999999);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 14999999999);

        assert!(stake::get_remaining_lockup_secs(pool_address) == LOCKUP_CYCLE_SECONDS - EPOCH_DURATION, 0);
        end_aptos_epoch(); // additionally forwards EPOCH_DURATION seconds

        // pending_inactive stake should have not been inactivated
        // 15100000001 * 1.01 active stake + 14999999999 pending_inactive * 1.01 stake
        assert_delegation(validator_address, pool_address, 15251000001, 0, 15149999998);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 15149999998);
        stake::assert_stake_pool(pool_address, 15251000001, 0, 0, 15149999998);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 3 * EPOCH_DURATION);
        end_aptos_epoch(); // additionally forwards EPOCH_DURATION seconds and expires lockup cycle

        // 15251000001 * 1.01 active stake + 15149999998 * 1.01 pending_inactive(now inactive) stake
        assert_delegation(validator_address, pool_address, 15403510001, 15301499997, 0);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 15301499997);
        stake::assert_stake_pool(pool_address, 15403510001, 15301499997, 0, 0);

        // add 50 coins from another account
        stake::mint(delegator, 50 * ONE_APT);
        add_stake(delegator, pool_address, 50 * ONE_APT);

        // observed lockup cycle should have advanced at `add_stake`(on synchronization)
        assert!(observed_lockup_cycle(pool_address) == 1, 0);

        fee = get_add_stake_fee(pool_address, 50 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 4999999999 - fee, 0, 0);
        assert_delegation(validator_address, pool_address, 15403510001, 15301499997, 0);
        stake::assert_stake_pool(pool_address, 15403510001, 15301499997, 50 * ONE_APT, 0);

        // cannot withdraw stake unlocked by others
        withdraw(delegator, pool_address, 50 * ONE_APT);
        assert!(coin::balance<AptosCoin>(delegator_address) == 0, 0);

        // withdraw own unlocked stake
        withdraw(validator, pool_address, 15301499997);
        assert!(coin::balance<AptosCoin>(validator_address) == 15301499997, 0);
        assert_delegation(validator_address, pool_address, 15403510001, 0, 0);
        // pending withdrawal has been executed and deleted
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        // inactive shares pool on OLC 0 has been deleted because its stake has been withdrawn
        assert_inactive_shares_pool(pool_address, 0, false, 0);

        // new pending withdrawal can be created on lockup cycle 1
        unlock(validator, pool_address, 5403510001);
        assert_delegation(validator_address, pool_address, 10000000000, 0, 5403510000);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 5403510000);

        // end lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 10000000000 * 1.01 active stake + 5403510000 * 1.01 pending_inactive(now inactive) stake
        assert_delegation(validator_address, pool_address, 10100000000, 5457545100, 0);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, true, 5457545100);

        // unlock when the pending withdrawal exists and gets automatically executed
        let balance = coin::balance<AptosCoin>(validator_address);
        unlock(validator, pool_address, 10100000000);
        assert!(coin::balance<AptosCoin>(validator_address) == balance + 5457545100, 0);
        assert_delegation(validator_address, pool_address, 0, 0, 10100000000);
        // this is the new pending withdrawal replacing the executed one
        assert_pending_withdrawal(validator_address, pool_address, true, 2, false, 10100000000);

        // create dummy validator to ensure the existing validator can leave the set
        initialize_test_validator(delegator, 100 * ONE_APT, true, true);
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();

        // expire lockup cycle on the stake pool
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        let observed_lockup_cycle = observed_lockup_cycle(pool_address);
        end_aptos_epoch();

        // observed lockup cycle should be unchanged as no stake has been inactivated
        synchronize_delegation_pool(pool_address);
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // stake is pending_inactive as it has not been inactivated
        stake::assert_stake_pool(pool_address, 5100500001, 0, 0, 10303010000);
        // 10100000000 * 1.01 * 1.01 pending_inactive stake
        assert_delegation(validator_address, pool_address, 0, 0, 10303010000);
        // the pending withdrawal should be reported as still pending
        assert_pending_withdrawal(validator_address, pool_address, true, 2, false, 10303010000);

        // validator is inactive and lockup expired => pending_inactive stake is withdrawable
        balance = coin::balance<AptosCoin>(validator_address);
        withdraw(validator, pool_address, 10303010000);

        assert!(coin::balance<AptosCoin>(validator_address) == balance + 10303010000, 0);
        assert_delegation(validator_address, pool_address, 0, 0, 0);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        stake::assert_stake_pool(pool_address, 5100500001, 0, 0, 0);
        // pending_inactive shares pool has not been deleted (as can still `unlock` this OLC)
        assert_inactive_shares_pool(pool_address, observed_lockup_cycle(pool_address), true, 0);

        stake::mint(validator, 30 * ONE_APT);
        add_stake(validator, pool_address, 30 * ONE_APT);
        unlock(validator, pool_address, 10 * ONE_APT);

        assert_delegation(validator_address, pool_address, 1999999999, 0, 1000000000);
        // the pending withdrawal should be reported as still pending
        assert_pending_withdrawal(validator_address, pool_address, true, 2, false, 1000000000);

        balance = coin::balance<AptosCoin>(validator_address);
        // pending_inactive balance would be under threshold => redeem entire balance
        withdraw(validator, pool_address, 1);
        // pending_inactive balance has been withdrawn and the pending withdrawal executed
        assert_delegation(validator_address, pool_address, 1999999999, 0, 0);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        assert!(coin::balance<AptosCoin>(validator_address) == balance + 1000000000, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_total_coins_inactive(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 100 * ONE_APT);
        stake::mint(delegator2, 200 * ONE_APT);
        add_stake(delegator1, pool_address, 100 * ONE_APT);
        add_stake(delegator2, pool_address, 200 * ONE_APT);
        end_aptos_epoch();

        assert_delegation(delegator1_address, pool_address, 100 * ONE_APT, 0, 0);
        assert_delegation(delegator2_address, pool_address, 200 * ONE_APT, 0, 0);

        // unlock some stake from delegator 1
        unlock(delegator1, pool_address, 50 * ONE_APT);
        assert_delegation(delegator1_address, pool_address, 5000000000, 0, 4999999999);

        // move to lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // delegator 1 pending_inactive stake has been inactivated
        assert_delegation(delegator1_address, pool_address, 5050000000, 5049999998, 0);
        assert_delegation(delegator2_address, pool_address, 202 * ONE_APT, 0, 0);

        synchronize_delegation_pool(pool_address);
        assert!(total_coins_inactive(pool_address) == 5049999998, 0);

        // unlock some stake from delegator 2
        unlock(delegator2, pool_address, 50 * ONE_APT);
        assert_delegation(delegator2_address, pool_address, 15200000001, 0, 4999999999);

        // withdraw some of inactive stake of delegator 1
        withdraw(delegator1, pool_address, 2049999998);
        assert_delegation(delegator1_address, pool_address, 5050000000, 3000000001, 0);
        assert!(total_coins_inactive(pool_address) == 3000000001, 0);

        // move to lockup cycle 2
        let (_, inactive, _, pending_inactive) = stake::get_stake(pool_address);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // delegator 2 pending_inactive stake has been inactivated
        assert_delegation(delegator1_address, pool_address, 5100500000, 3000000001, 0);
        assert_delegation(delegator2_address, pool_address, 15352000001, 5049999998, 0);

        // total_coins_inactive remains unchanged in the absence of user operations
        assert!(total_coins_inactive(pool_address) == inactive, 0);
        synchronize_delegation_pool(pool_address);
        // total_coins_inactive == previous inactive stake + previous pending_inactive stake and its rewards
        assert!(total_coins_inactive(pool_address) == inactive + pending_inactive + pending_inactive / 100, 0);

        // withdraw some of inactive stake of delegator 2
        let total_coins_inactive = total_coins_inactive(pool_address);
        withdraw(delegator2, pool_address, 3049999998);
        assert!(total_coins_inactive(pool_address) == total_coins_inactive - 3049999997, 0);

        // unlock some stake from delegator `validator`
        unlock(validator, pool_address, 50 * ONE_APT);

        // create dummy validator to ensure the existing validator can leave the set
        initialize_test_validator(delegator1, 100 * ONE_APT, true, true);
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();

        // move to lockup cycle 3
        (_, inactive, _, pending_inactive) = stake::get_stake(pool_address);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // pending_inactive stake has not been inactivated as validator is inactive
        let (_, inactive_now, _, pending_inactive_now) = stake::get_stake(pool_address);
        assert!(inactive_now == inactive, inactive_now);
        assert!(pending_inactive_now == pending_inactive, pending_inactive_now);

        // total_coins_inactive remains unchanged in the absence of a new OLC
        synchronize_delegation_pool(pool_address);
        assert!(total_coins_inactive(pool_address) == inactive, 0);

        // withdraw entire pending_inactive stake
        withdraw(validator, pool_address, MAX_U64);
        assert!(total_coins_inactive(pool_address) == inactive, 0);
        (_, _, _, pending_inactive) = stake::get_stake(pool_address);
        assert!(pending_inactive == 0, pending_inactive);

        // withdraw entire inactive stake
        withdraw(delegator1, pool_address, MAX_U64);
        withdraw(delegator2, pool_address, MAX_U64);
        assert!(total_coins_inactive(pool_address) == 0, 0);
        (_, inactive, _, _) = stake::get_stake(pool_address);
        assert!(inactive == 0, inactive);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_reactivate_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // unlock some stake from the active one
        unlock(validator, pool_address, 100 * ONE_APT);
        assert_delegation(validator_address, pool_address, 100 * ONE_APT, 0, 100 * ONE_APT);
        stake::assert_stake_pool(pool_address, 100 * ONE_APT, 0, 0, 100 * ONE_APT);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 100 * ONE_APT);

        // add some stake to pending_active state
        stake::mint(validator, 150 * ONE_APT);
        add_stake(validator, pool_address, 150 * ONE_APT);

        let fee = get_add_stake_fee(pool_address, 150 * ONE_APT);
        assert_delegation(validator_address, pool_address, 250 * ONE_APT - fee, 0, 100 * ONE_APT);
        stake::assert_stake_pool(pool_address, 100 * ONE_APT, 0, 150 * ONE_APT, 100 * ONE_APT);

        // can reactivate only pending_inactive stake
        reactivate_stake(validator, pool_address, 150 * ONE_APT);

        assert_delegation(validator_address, pool_address, 350 * ONE_APT - fee, 0, 0);
        stake::assert_stake_pool(pool_address, 200 * ONE_APT, 0, 150 * ONE_APT, 0);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);

        end_aptos_epoch();
        // 20000000000 active stake * 1.01 + 15000000000 pending_active stake
        assert_delegation(validator_address, pool_address, 35200000000, 0, 0);

        // unlock stake added at previous epoch (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150 * ONE_APT);
        assert_delegation(validator_address, pool_address, 20200000001, 0, 14999999999);
        stake::assert_stake_pool(pool_address, 20200000001, 0, 0, 14999999999);

        // inactivate pending_inactive stake
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 20200000001 active stake * 1.01 + 14999999999 pending_inactive stake * 1.01
        assert_delegation(validator_address, pool_address, 20402000001, 15149999998, 0);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 15149999998);

        // cannot reactivate inactive stake
        reactivate_stake(validator, pool_address, 15149999998);
        assert_delegation(validator_address, pool_address, 20402000001, 15149999998, 0);

        // unlock stake in the new lockup cycle (the pending withdrawal is executed)
        unlock(validator, pool_address, 100 * ONE_APT);
        assert!(coin::balance<AptosCoin>(validator_address) == 15149999998, 0);
        assert_delegation(validator_address, pool_address, 10402000002, 0, 9999999999);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 9999999999);

        // reactivate the new pending withdrawal almost entirely
        reactivate_stake(validator, pool_address, 8999999999);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 1000000000);
        // reactivate remaining stake of the new pending withdrawal
        reactivate_stake(validator, pool_address, 1000000000);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_withdraw_many(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_APT);
        add_stake(delegator, pool_address, 200 * ONE_APT);

        unlock(validator, pool_address, 100 * ONE_APT);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 100 * ONE_APT);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_delegation(delegator_address, pool_address, 200 * ONE_APT, 0, 0);
        assert_delegation(validator_address, pool_address, 90899999999, 10100000000, 0);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 10100000000);
        assert_inactive_shares_pool(pool_address, 0, true, 100 * ONE_APT);

        // check cannot withdraw inactive stake unlocked by others
        withdraw(delegator, pool_address, MAX_U64);
        assert_delegation(delegator_address, pool_address, 200 * ONE_APT, 0, 0);
        assert_delegation(validator_address, pool_address, 90899999999, 10100000000, 0);

        unlock(delegator, pool_address, 100 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 10000000000, 0, 9999999999);
        assert_delegation(validator_address, pool_address, 90900000000, 10100000000, 0);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 9999999999);

        // check cannot withdraw inactive stake unlocked by others even if owning pending_inactive
        withdraw(delegator, pool_address, MAX_U64);
        assert_delegation(delegator_address, pool_address, 10000000000, 0, 9999999999);
        assert_delegation(validator_address, pool_address, 90900000000, 10100000000, 0);

        // withdraw entire owned inactive stake
        let balance = coin::balance<AptosCoin>(validator_address);
        withdraw(validator, pool_address, MAX_U64);
        assert!(coin::balance<AptosCoin>(validator_address) == balance + 10100000000, 0);
        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        assert_inactive_shares_pool(pool_address, 0, false, 0);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_delegation(delegator_address, pool_address, 10100000000, 10099999998, 0);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, true, 10099999998);
        assert_inactive_shares_pool(pool_address, 1, true, 9999999999);

        // use too small of an unlock amount to actually transfer shares to the pending_inactive pool
        // check that no leftovers have been produced on the stake or delegation pools
        stake::assert_stake_pool(pool_address, 101909000001, 10099999998, 0, 0);
        unlock_with_min_stake_disabled(delegator, pool_address, 1);
        stake::assert_stake_pool(pool_address, 101909000001, 10099999998, 0, 0);
        assert_delegation(delegator_address, pool_address, 10100000000, 10099999998, 0);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, true, 10099999998);

        // implicitly execute the pending withdrawal by unlocking min stake to buy 1 share
        unlock_with_min_stake_disabled(delegator, pool_address, 2);
        stake::assert_stake_pool(pool_address, 101909000000, 0, 0, 1);
        assert_delegation(delegator_address, pool_address, 10099999998, 0, 1);
        // old pending withdrawal has been replaced
        assert_pending_withdrawal(delegator_address, pool_address, true, 2, false, 1);
        assert_inactive_shares_pool(pool_address, 1, false, 0);
        assert_inactive_shares_pool(pool_address, 2, true, 1);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_inactivate_no_excess_stake(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1200 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_APT);
        add_stake(delegator, pool_address, 200 * ONE_APT);

        // create inactive and pending_inactive stakes on the stake pool
        unlock(validator, pool_address, 200 * ONE_APT);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(delegator, pool_address, 100 * ONE_APT);

        // check no excess pending_inactive is inactivated in the special case
        // the validator had gone inactive before its lockup expired

        let observed_lockup_cycle = observed_lockup_cycle(pool_address);

        // create dummy validator to ensure the existing validator can leave the set
        initialize_test_validator(delegator, 100 * ONE_APT, true, true);
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE, 0);

        // expire lockup afterwards
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        synchronize_delegation_pool(pool_address);
        // no new inactive stake detected => OLC does not advance
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // pending_inactive stake has not been inactivated
        stake::assert_stake_pool(pool_address, 113231100001, 20200000000, 0, 10200999997);
        assert_delegation(delegator_address, pool_address, 10201000000, 0, 10200999997);
        assert_delegation(validator_address, pool_address, 103030100000, 20200000000, 0);

        // withdraw some inactive stake (remaining pending_inactive is not inactivated)
        withdraw(validator, pool_address, 200000000);
        stake::assert_stake_pool(pool_address, 113231100001, 20000000001, 0, 10200999997);
        assert_delegation(delegator_address, pool_address, 10201000000, 0, 10200999997);
        assert_delegation(validator_address, pool_address, 103030100000, 20000000001, 0);

        // withdraw some pending_inactive stake (remaining pending_inactive is not inactivated)
        withdraw(delegator, pool_address, 200999997);
        stake::assert_stake_pool(pool_address, 113231100001, 20000000001, 0, 10000000001);
        assert_delegation(delegator_address, pool_address, 10201000000, 0, 10000000001);
        assert_delegation(validator_address, pool_address, 103030100000, 20000000001, 0);

        // no new inactive stake detected => OLC does not advance
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        unlock(delegator, pool_address, 10201000000);
        withdraw(delegator, pool_address, 10201000000);
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        assert_delegation(delegator_address, pool_address, 0, 0, 10000000002);
        assert_delegation(validator_address, pool_address, 103030100001, 20000000001, 0);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 20000000001);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 10000000002);
        stake::assert_stake_pool(pool_address, 103030100001, 20000000001, 0, 10000000002);

        // reactivate validator
        stake::join_validator_set(validator, pool_address);
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_PENDING_ACTIVE, 0);
        end_aptos_epoch();

        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 0);
        // no rewards have been produced yet and no stake inactivated as lockup has been refreshed
        stake::assert_stake_pool(pool_address, 103030100001, 20000000001, 0, 10000000002);

        synchronize_delegation_pool(pool_address);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 20000000001);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 10000000002);
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // cannot withdraw pending_inactive stake anymore
        withdraw(delegator, pool_address, 10000000002);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 10000000002);

        // earning rewards is resumed from this epoch on
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 104060401001, 20000000001, 0, 10100000002);

        // new pending_inactive stake earns rewards but so does the old one
        unlock(validator, pool_address, 104060401001);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 104060401000);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 10100000002);
        end_aptos_epoch();
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 105101005010);
        assert_pending_withdrawal(delegator_address, pool_address, true, 1, false, 10201000002);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_active_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        // 100000000000 active stake * 1.01
        assert_delegation(validator_address, pool_address, 1010 * ONE_APT, 0, 0);

        // add stake in pending_active state
        stake::mint(validator, 200 * ONE_APT);
        add_stake(validator, pool_address, 200 * ONE_APT);

        let fee = get_add_stake_fee(pool_address, 200 * ONE_APT);
        assert_delegation(validator_address, pool_address, 1210 * ONE_APT - fee, 0, 0);

        end_aptos_epoch();
        // 101000000000 active stake * 1.01 + 20000000000 pending_active stake with no rewards
        assert_delegation(validator_address, pool_address, 122010000000, 0, 0);

        end_aptos_epoch();
        // 122010000000 active stake * 1.01
        assert_delegation(validator_address, pool_address, 123230100000, 0, 0);

        // 123230100000 active stake * 1.01
        end_aptos_epoch();
        // 124462401000 active stake * 1.01
        end_aptos_epoch();
        // 125707025010 active stake * 1.01
        end_aptos_epoch();
        // 126964095260 active stake * 1.01
        end_aptos_epoch();
        // 128233736212 active stake * 1.01
        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 129516073574, 0, 0);

        // unlock 200 coins from delegator `validator`
        unlock(validator, pool_address, 200 * ONE_APT);
        assert_delegation(validator_address, pool_address, 109516073575, 0, 19999999999);

        // end this lockup cycle
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // 109516073575 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        assert_delegation(validator_address, pool_address, 110611234310, 20199999998, 0);

        end_aptos_epoch();
        // 110611234310 active stake * 1.01 + 20199999998 inactive stake
        assert_delegation(validator_address, pool_address, 111717346653, 20199999998, 0);

        // add stake in pending_active state
        stake::mint(validator, 1000 * ONE_APT);
        add_stake(validator, pool_address, 1000 * ONE_APT);

        fee = get_add_stake_fee(pool_address, 1000 * ONE_APT);
        assert_delegation(validator_address, pool_address, 211717346653 - fee, 20199999998, 0);

        end_aptos_epoch();
        // 111717346653 active stake * 1.01 + 100000000000 pending_active stake + 20199999998 inactive stake
        assert_delegation(validator_address, pool_address, 212834520119, 20199999998, 0);

        end_aptos_epoch();
        // 212834520119 active stake * 1.01 + 20199999998 inactive stake
        assert_delegation(validator_address, pool_address, 214962865320, 20199999998, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_active_stake_rewards_multiple(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add stake in pending_active state
        stake::mint(delegator, 300 * ONE_APT);
        add_stake(delegator, pool_address, 300 * ONE_APT);

        let fee = get_add_stake_fee(pool_address, 300 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 300 * ONE_APT - fee, 0, 0);
        assert_delegation(validator_address, pool_address, 200 * ONE_APT, 0, 0);
        stake::assert_stake_pool(pool_address, 200 * ONE_APT, 0, 300 * ONE_APT, 0);

        end_aptos_epoch();
        // `delegator` got its `add_stake` fee back and `validator` its active stake rewards
        assert_delegation(delegator_address, pool_address, 300 * ONE_APT, 0, 0);
        assert_delegation(validator_address, pool_address, 20199999999, 0, 0);
        stake::assert_stake_pool(pool_address, 502 * ONE_APT, 0, 0, 0);

        // delegators earn their own rewards from now on
        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 303 * ONE_APT, 0, 0);
        assert_delegation(validator_address, pool_address, 20401999999, 0, 0);
        stake::assert_stake_pool(pool_address, 50702000000, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 30603000000, 0, 0);
        assert_delegation(validator_address, pool_address, 20606019999, 0, 0);
        stake::assert_stake_pool(pool_address, 51209020000, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 30909030000, 0, 0);
        assert_delegation(validator_address, pool_address, 20812080199, 0, 0);
        stake::assert_stake_pool(pool_address, 51721110200, 0, 0, 0);

        // add more stake in pending_active state than currently active
        stake::mint(delegator, 1000 * ONE_APT);
        add_stake(delegator, pool_address, 1000 * ONE_APT);

        fee = get_add_stake_fee(pool_address, 1000 * ONE_APT);
        assert_delegation(delegator_address, pool_address, 130909030000 - fee, 0, 0);
        assert_delegation(validator_address, pool_address, 20812080199, 0, 0);

        end_aptos_epoch();
        // `delegator` got its `add_stake` fee back and `validator` its active stake rewards
        assert_delegation(delegator_address, pool_address, 131218120300, 0, 0);
        assert_delegation(validator_address, pool_address, 21020201001, 0, 0);
        stake::assert_stake_pool(pool_address, 152238321302, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_pending_inactive_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 1010 * ONE_APT, 0, 0);

        // unlock 200 coins from delegator `validator`
        unlock(validator, pool_address, 200 * ONE_APT);
        assert_delegation(validator_address, pool_address, 81000000001, 0, 19999999999);

        end_aptos_epoch(); // 81000000001 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        end_aptos_epoch(); // 81810000001 active stake * 1.01 + 20199999998 pending_inactive stake * 1.01

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 82628100001 active stake * 1.01 + 20401999997 pending_inactive stake * 1.01
        end_aptos_epoch(); // 83454381001 active stake * 1.01 + 20606019996 pending_inactive stake(now inactive)
        assert_delegation(validator_address, pool_address, 84288924811, 20606019996, 0);

        // unlock 200 coins from delegator `validator` which implicitly executes its pending withdrawal
        unlock(validator, pool_address, 200 * ONE_APT);
        assert!(coin::balance<AptosCoin>(validator_address) == 20606019996, 0);
        assert_delegation(validator_address, pool_address, 64288924812, 0, 19999999999);

        // lockup cycle is not ended, pending_inactive stake is still earning
        end_aptos_epoch(); // 64288924812 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        end_aptos_epoch(); // 64931814060 active stake * 1.01 + 20199999998 pending_inactive stake * 1.01
        end_aptos_epoch(); // 65581132200 active stake * 1.01 + 20401999997 pending_inactive stake * 1.01
        end_aptos_epoch(); // 66236943522 active stake * 1.01 + 20606019996 pending_inactive stake * 1.01
        assert_delegation(validator_address, pool_address, 66899312957, 0, 20812080195);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 66899312957 active stake * 1.01 + 20812080195 pending_inactive stake * 1.01
        end_aptos_epoch(); // 67568306086 active stake * 1.01 + 21020200996 pending_inactive stake(now inactive)
        end_aptos_epoch(); // 68243989147 active stake * 1.01 + 21020200996 inactive stake
        assert_delegation(validator_address, pool_address, 68926429037, 21020200996, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_out_of_order_redeem(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000 * ONE_APT, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 300 * ONE_APT);
        add_stake(delegator1, pool_address, 300 * ONE_APT);

        stake::mint(delegator2, 300 * ONE_APT);
        add_stake(delegator2, pool_address, 300 * ONE_APT);

        end_aptos_epoch();

        // create the pending withdrawal of delegator 1 in lockup cycle 0
        unlock(delegator1, pool_address, 150 * ONE_APT);
        assert_pending_withdrawal(delegator1_address, pool_address, true, 0, false, 14999999999);

        // move to lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // create the pending withdrawal of delegator 2 in lockup cycle 1
        unlock(delegator2, pool_address, 150 * ONE_APT);
        assert_pending_withdrawal(delegator2_address, pool_address, true, 1, false, 14999999999);
        // 14999999999 pending_inactive stake * 1.01
        assert_pending_withdrawal(delegator1_address, pool_address, true, 0, true, 15149999998);

        // move to lockup cycle 2
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_pending_withdrawal(delegator2_address, pool_address, true, 1, true, 15149999998);
        assert_pending_withdrawal(delegator1_address, pool_address, true, 0, true, 15149999998);

        // both delegators who unlocked at different lockup cycles should be able to withdraw their stakes
        withdraw(delegator1, pool_address, 15149999998);
        withdraw(delegator2, pool_address, 5149999998);

        assert_pending_withdrawal(delegator2_address, pool_address, true, 1, true, 10000000001);
        assert_pending_withdrawal(delegator1_address, pool_address, false, 0, false, 0);
        assert!(coin::balance<AptosCoin>(delegator1_address) == 15149999998, 0);
        assert!(coin::balance<AptosCoin>(delegator2_address) == 5149999997, 0);

        // recreate the pending withdrawal of delegator 1 in lockup cycle 2
        unlock(delegator1, pool_address, 100 * ONE_APT);

        // move to lockup cycle 3
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_pending_withdrawal(delegator2_address, pool_address, true, 1, true, 10000000001);
        // 9999999999 pending_inactive stake * 1.01
        assert_pending_withdrawal(delegator1_address, pool_address, true, 2, true, 10099999998);

        // withdraw inactive stake of delegator 2 left from lockup cycle 1 in cycle 3
        withdraw(delegator2, pool_address, 10000000001);
        assert!(coin::balance<AptosCoin>(delegator2_address) == 15149999998, 0);
        assert_pending_withdrawal(delegator2_address, pool_address, false, 0, false, 0);

        // withdraw inactive stake of delegator 1 left from previous lockup cycle
        withdraw(delegator1, pool_address, 10099999998);
        assert!(coin::balance<AptosCoin>(delegator1_address) == 15149999998 + 10099999998, 0);
        assert_pending_withdrawal(delegator1_address, pool_address, false, 0, false, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_operator_fee(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);

        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(validator, 1265, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);
        assert!(stake::get_operator(pool_address) == validator_address, 0);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 100 * ONE_APT);
        add_stake(delegator1, pool_address, 100 * ONE_APT);

        stake::mint(delegator2, 200 * ONE_APT);
        add_stake(delegator2, pool_address, 200 * ONE_APT);

        // validator is inactive and added stake is instantly `active`
        stake::assert_stake_pool(pool_address, 300 * ONE_APT, 0, 0, 0);

        // validator does not produce rewards yet
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 300 * ONE_APT, 0, 0, 0);

        // therefore, there are no operator commission rewards yet
        assert_delegation(validator_address, pool_address, 0, 0, 0);

        // activate validator
        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // produce active rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30300000000, 0, 0, 0);

        // 300000000 active rewards * 0.1265
        assert_delegation(validator_address, pool_address, 37950000, 0, 0);
        // 10000000000 active stake * (1 + 1% reward-rate * 0.8735)
        assert_delegation(delegator1_address, pool_address, 10087350000, 0, 0);
        // 20000000000 active stake * 1.008735
        assert_delegation(delegator2_address, pool_address, 20174700000, 0, 0);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30603000000, 0, 0, 0);

        // 603000000 active rewards * 0.1265 instead of
        // 303000000 active rewards * 0.1265 + 37950000 active stake * 1.008735
        // because operator commission rewards are not automatically restaked compared to already owned stake
        assert_delegation(validator_address, pool_address, 76279500, 0, 0);
        // 10087350000 active stake * 1.008735 + some of the rewards of previous commission if restaked
        assert_delegation(delegator1_address, pool_address, 10175573500, 0, 0);
        // 20174700000 active stake * 1.008735 + some of the rewards of previous commission if restaked
        assert_delegation(delegator2_address, pool_address, 20351147000, 0, 0);

        // restake operator commission rewards
        synchronize_delegation_pool(pool_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30909030000, 0, 0, 0);

        // 306030000 active rewards * 0.1265 + 76279500 active stake * 1.008735
        assert_delegation(validator_address, pool_address, 115658596, 0, 0);
        // 10175573500 active stake * 1.008735
        assert_delegation(delegator1_address, pool_address, 10264457134, 0, 0);
        // 20351147000 active stake * 1.008735
        assert_delegation(delegator2_address, pool_address, 20528914269, 0, 0);

        // check operator is rewarded by pending_inactive stake too
        unlock(delegator2, pool_address, 100 * ONE_APT);
        stake::assert_stake_pool(pool_address, 20909030001, 0, 0, 9999999999);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 21118120301, 0, 0, 10099999998);

        assert_pending_withdrawal(validator_address, pool_address, false, 0, false, 0);
        // distribute operator pending_inactive commission rewards
        synchronize_delegation_pool(pool_address);
        // 99999999 pending_inactive rewards * 0.1265
        assert_pending_withdrawal(validator_address, pool_address, true, 0, false, 12649998);

        // 209090300 active rewards * 0.1265 + 115658596 active stake * 1.008735
        // 99999999 pending_inactive rewards * 0.1265
        assert_delegation(validator_address, pool_address, 143118796, 0, 12649998);
        // 10264457134 active stake * 1.008735
        assert_delegation(delegator1_address, pool_address, 10354117168, 0, 0);
        // 10528914270 active stake * 1.008735
        // 9999999999 pending_inactive stake * 1.008735
        assert_delegation(delegator2_address, pool_address, 10620884336, 0, 10087349999);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 21329301504, 10200999997, 0, 0);

        // operator pending_inactive rewards on previous epoch have been inactivated
        // 211181203 active rewards * 0.1265 + 143118796 active stake * 1.008735
        // 100999999 pending_inactive rewards * 0.1265 + 12649998 pending_inactive stake * 1.008735
        assert_delegation(validator_address, pool_address, 171083360, 25536995, 0);
        // distribute operator pending_inactive commission rewards
        synchronize_delegation_pool(pool_address);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 25536995);

        // check operator is not rewarded by `add_stake` fees
        stake::mint(delegator1, 100 * ONE_APT);
        assert!(get_add_stake_fee(pool_address, 100 * ONE_APT) > 0, 0);
        add_stake(delegator1, pool_address, 100 * ONE_APT);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 31542594519, 10200999997, 0, 0);

        // 213293015 active rewards * 0.1265 + 171083360 active stake * 1.008735
        assert_delegation(validator_address, pool_address, 199559340, 25536995, 0);

        // unlock some more stake to produce pending_inactive commission
        // 10620884336 active stake * (1.008735 ^ 2 epochs)
        // 10087349999 pending_inactive stake * 1.008735
        assert_delegation(delegator2_address, pool_address, 10807241561, 10175463001, 0);
        unlock(delegator2, pool_address, 100 * ONE_APT);
        // 10807241561 - 100 APT < `MIN_COINS_ON_SHARES_POOL` thus active stake is entirely unlocked
        assert_delegation(delegator2_address, pool_address, 0, 0, 10807241561);
        end_aptos_epoch();

        // in-flight pending_inactive commission can coexist with previous inactive commission
        assert_delegation(validator_address, pool_address, 227532711, 25536996, 13671160);
        assert_pending_withdrawal(validator_address, pool_address, true, 0, true, 25536996);

        // distribute in-flight pending_inactive commission, implicitly executing the inactive withdrawal of operator
        coin::register<AptosCoin>(validator);
        synchronize_delegation_pool(pool_address);
        assert!(coin::balance<AptosCoin>(validator_address) == 25536996, 0);

        // in-flight commission has been synced, implicitly used to buy shares for operator
        // expect operator stake to be slightly less than previously reported by `Self::get_stake`
        assert_delegation(validator_address, pool_address, 227532711, 0, 13671159);
        assert_pending_withdrawal(validator_address, pool_address, true, 1, false, 13671159);
    }

    #[test(aptos_framework = @aptos_framework, old_operator = @0x123, delegator = @0x010, new_operator = @0x020)]
    public entry fun test_change_operator(
        aptos_framework: &signer,
        old_operator: &signer,
        delegator: &signer,
        new_operator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let old_operator_address = signer::address_of(old_operator);
        account::create_account_for_test(old_operator_address);

        let new_operator_address = signer::address_of(new_operator);
        account::create_account_for_test(new_operator_address);

        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(old_operator, 1265, vector::empty<u8>());
        let pool_address = get_owned_pool_address(old_operator_address);
        assert!(stake::get_operator(pool_address) == old_operator_address, 0);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_APT);
        add_stake(delegator, pool_address, 200 * ONE_APT);
        unlock(delegator, pool_address, 100 * ONE_APT);

        // activate validator
        stake::rotate_consensus_key(old_operator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);
        stake::join_validator_set(old_operator, pool_address);
        end_aptos_epoch();

        // produce active and pending_inactive rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10100000000, 0, 0, 10100000000);
        assert_delegation(old_operator_address, pool_address, 12650000, 0, 12650000);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10201000000, 0, 0, 10201000000);
        assert_delegation(old_operator_address, pool_address, 25426500, 0, 25426500);

        // change operator
        set_operator(old_operator, new_operator_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10303010000, 0, 0, 10303010000);
        // 25426500 active stake * 1.008735 and 25426500 pending_inactive stake * 1.008735
        assert_delegation(old_operator_address, pool_address, 25648600, 0, 25648600);
        // 102010000 active rewards * 0.1265 and 102010000 pending_inactive rewards * 0.1265
        assert_delegation(new_operator_address, pool_address, 12904265, 0, 12904265);

        // restake `new_operator` commission rewards
        synchronize_delegation_pool(pool_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10406040100, 0, 0, 10406040100);
        // 25648600 active stake * 1.008735 and 25648600 pending_inactive stake * 1.008735
        assert_delegation(old_operator_address, pool_address, 25872641, 0, 25872641);
        // 103030100 active rewards * 0.1265 and 12904265 active stake * 1.008735
        // 103030100 pending_inactive rewards * 0.1265 and 12904265 pending_inactive stake * 1.008735
        assert_delegation(new_operator_address, pool_address, 26050290, 0, 26050290);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_min_stake_is_preserved(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 100 * ONE_APT, true, false);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        // add stake without fees as validator is not active yet
        stake::mint(delegator1, 50 * ONE_APT);
        add_stake(delegator1, pool_address, 50 * ONE_APT);
        stake::mint(delegator2, 16 * ONE_APT);
        add_stake(delegator2, pool_address, 16 * ONE_APT);

        // validator becomes active and share price is 1
        end_aptos_epoch();

        assert_delegation(delegator1_address, pool_address, 5000000000, 0, 0);
        // pending_inactive balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL - 1);
        assert_delegation(delegator1_address, pool_address, 3999999999, 0, 1000000001);

        // pending_inactive balance is over threshold
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 4000000000, 0, 1000000000);

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 5000000000, 0, 0);

        // active balance would be under threshold => move entire balance
        unlock(delegator1, pool_address, 5000000000 - (MIN_COINS_ON_SHARES_POOL - 1));
        assert_delegation(delegator1_address, pool_address, 0, 0, 5000000000);

        // active balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 1000000001, 0, 3999999999);

        // active balance is over threshold
        unlock(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 1000000000, 0, 4000000000);

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, 4000000000 - (MIN_COINS_ON_SHARES_POOL - 1));
        assert_delegation(delegator1_address, pool_address, 5000000000, 0, 0);

        // active + pending_inactive balance < 2 * MIN_COINS_ON_SHARES_POOL
        // stake can live on only one of the shares pools
        assert_delegation(delegator2_address, pool_address, 16 * ONE_APT, 0, 0);
        unlock(delegator2, pool_address, 1);
        assert_delegation(delegator2_address, pool_address, 0, 0, 16 * ONE_APT);
        reactivate_stake(delegator2, pool_address, 1);
        assert_delegation(delegator2_address, pool_address, 16 * ONE_APT, 0, 0);

        unlock(delegator2, pool_address, ONE_APT);
        assert_delegation(delegator2_address, pool_address, 0, 0, 16 * ONE_APT);
        reactivate_stake(delegator2, pool_address, 2 * ONE_APT);
        assert_delegation(delegator2_address, pool_address, 16 * ONE_APT, 0, 0);

        // share price becomes 1.01 on both pools
        unlock(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 3999999999, 0, 1000000001);
        end_aptos_epoch();
        assert_delegation(delegator1_address, pool_address, 4039999998, 0, 1010000001);

        // pending_inactive balance is over threshold
        reactivate_stake(delegator1, pool_address, 10000001);
        assert_delegation(delegator1_address, pool_address, 4049999998, 0, 1000000001);

        // 1 coin < 1.01 so no shares are redeemed
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 4049999998, 0, 1000000001);

        // pending_inactive balance is over threshold
        // requesting 2 coins actually redeems 1 coin from pending_inactive pool
        reactivate_stake(delegator1, pool_address, 2);
        assert_delegation(delegator1_address, pool_address, 4049999999, 0, 1000000000);

        // 1 coin < 1.01 so no shares are redeemed
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 4049999999, 0, 1000000000);

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, 2);
        assert_delegation(delegator1_address, pool_address, 5049999999, 0, 0);

        // pending_inactive balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL - 1);
        assert_delegation(delegator1_address, pool_address, 4049999998, 0, 1000000000);

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, 1);
        assert_delegation(delegator1_address, pool_address, 5049999998, 0, 0);
    }

    #[test_only]
    public fun assert_delegation(
        delegator_address: address,
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_inactive_stake: u64,
    ) acquires DelegationPool {
        let (actual_active, actual_inactive, actual_pending_inactive) = get_stake(pool_address, delegator_address);
        assert!(actual_active == active_stake, actual_active);
        assert!(actual_inactive == inactive_stake, actual_inactive);
        assert!(actual_pending_inactive == pending_inactive_stake, actual_pending_inactive);
    }

    #[test_only]
    public fun assert_pending_withdrawal(
        delegator_address: address,
        pool_address: address,
        exists: bool,
        olc: u64,
        inactive: bool,
        stake: u64,
    ) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (withdrawal_exists, withdrawal_olc) = pending_withdrawal_exists(pool, delegator_address);
        assert!(withdrawal_exists == exists, 0);
        assert!(withdrawal_olc.index == olc, withdrawal_olc.index);
        let (withdrawal_inactive, withdrawal_stake) = get_pending_withdrawal(pool_address, delegator_address);
        assert!(withdrawal_inactive == inactive, 0);
        assert!(withdrawal_stake == stake, withdrawal_stake);
    }

    #[test_only]
    public fun assert_inactive_shares_pool(
        pool_address: address,
        olc: u64,
        exists: bool,
        stake: u64,
    ) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        assert!(table::contains(&pool.inactive_shares, olc_with_index(olc)) == exists, 0);
        if (exists) {
            let actual_stake = total_coins(table::borrow(&pool.inactive_shares, olc_with_index(olc)));
            assert!(actual_stake == stake, actual_stake);
        } else {
            assert!(0 == stake, 0);
        }
    }

    #[test_only]
    public fun total_coins_inactive(pool_address: address): u64 acquires DelegationPool {
        borrow_global<DelegationPool>(pool_address).total_coins_inactive
    }
}
