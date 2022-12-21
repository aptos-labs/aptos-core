module aptos_framework::delegation_pool {
    use std::error;
    use std::vector;

    use aptos_std::pool_u64_unbound as pool_u64;
    use aptos_std::table::{Self, Table};

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};

    friend aptos_framework::stake;
    friend aptos_framework::delegate;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 1;

    /// There is a pending withdrawal to be executed before unlocking any stake
    const EPENDING_WITHDRAWAL_EXISTS: u64 = 2;

    struct DelegationPool has key {
        active_shares: pool_u64::Pool,
        inactive_shares: vector<pool_u64::Pool>,
        pending_withdrawal: Table<address, u64>,
        stake_pool_signer_cap: account::SignerCapability,

        // The events emitted for delegation operations on the pool
        add_stake_events: EventHandle<AddStakeEvent>,
        reactivate_stake_events: EventHandle<ReactivateStakeEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: EventHandle<WithdrawStakeEvent>,
    }

    struct AddStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
    }

    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount: u64,
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

    public(friend) fun initialize(stake_pool_signer: &signer, stake_pool_signer_cap: account::SignerCapability) {
        move_to(stake_pool_signer, DelegationPool {
            active_shares: pool_u64::create(),
            inactive_shares: vector::singleton(pool_u64::create()),
            pending_withdrawal: table::new<address, u64>(),
            stake_pool_signer_cap,
            add_stake_events: account::new_event_handle<AddStakeEvent>(stake_pool_signer),
            reactivate_stake_events: account::new_event_handle<ReactivateStakeEvent>(stake_pool_signer),
            unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(stake_pool_signer),
            withdraw_stake_events: account::new_event_handle<WithdrawStakeEvent>(stake_pool_signer),
        });
    }

    public(friend) fun get_stake_pool_signer(pool_address: address): signer acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        account::create_signer_with_capability(&borrow_global<DelegationPool>(pool_address).stake_pool_signer_cap)
    }

    public fun delegation_pool_exists(addr: address): bool {
        exists<DelegationPool>(addr)
    }

    /// there are stake pools proxied by no delegation pool
    public fun assert_delegation_pool_exists(pool_address: address) {
        assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));
    }

    public fun current_lockup_epoch(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        current_lockup_epoch_internal(borrow_global<DelegationPool>(pool_address))
    }

    fun current_lockup_epoch_internal(pool: &DelegationPool): u64 {
        vector::length(&pool.inactive_shares) - 1
    }

    fun latest_inactive_shares_pool(pool: &mut DelegationPool): &mut pool_u64::Pool {
        let current_lockup_epoch = current_lockup_epoch_internal(pool);
        vector::borrow_mut(&mut pool.inactive_shares, current_lockup_epoch)
    }

    public entry fun pending_withdrawal_exists(
        pool_address: address,
        delegator_address: address,
    ): (bool, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        if (table::contains(&pool.pending_withdrawal, delegator_address)) {
            (true, *table::borrow(&pool.pending_withdrawal, delegator_address))
        } else {
            (false, 0)
        }
    }

    public(friend) fun emit_add_stake_event(
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
    ) acquires DelegationPool {
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        event::emit_event(
            &mut pool.add_stake_events,
            AddStakeEvent {
                pool_address,
                delegator_address,
                amount_added,
            },
        );
    }

    public(friend) fun emit_reactivate_stake_event(
        pool_address: address,
        delegator_address: address,
        amount: u64,
    ) acquires DelegationPool {
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        event::emit_event(
            &mut pool.reactivate_stake_events,
            ReactivateStakeEvent {
                pool_address,
                delegator_address,
                amount,
            },
        );
    }

    public(friend) fun emit_unlock_stake_event(
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64,
    ) acquires DelegationPool {
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        event::emit_event(
            &mut pool.unlock_stake_events,
            UnlockStakeEvent {
                pool_address,
                delegator_address,
                amount_unlocked,
            },
        );
    }

    public(friend) fun emit_withdraw_stake_event(
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64,
    ) acquires DelegationPool {
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        event::emit_event(
            &mut pool.withdraw_stake_events,
            WithdrawStakeEvent {
                pool_address,
                delegator_address,
                amount_withdrawn,
            },
        );
    }

    public(friend) fun buy_in_active_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        if (coins_amount == 0) return 0;
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        pool_u64::buy_in(&mut pool.active_shares, shareholder, coins_amount)
    }

    public(friend) fun buy_in_inactive_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        if (coins_amount == 0) return 0;
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        // save lockup epoch for new pending withdrawal if no existing previous one
        let current_lockup_epoch = current_lockup_epoch_internal(pool);
        assert!(*table::borrow_mut_with_default(
            &mut pool.pending_withdrawal,
            shareholder,
            current_lockup_epoch
        ) == current_lockup_epoch,
            error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)
        );

        // cannot buy inactive shares, only pending_inactive at current lockup epoch
        pool_u64::buy_in(latest_inactive_shares_pool(pool), shareholder, coins_amount)
    }

    fun amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        if (coins_amount >= pool_u64::balance(shares_pool, shareholder)) {
            // take into account rounding errors and extract entire shares amount
            pool_u64::shares(shares_pool, shareholder)
        } else {
            pool_u64::amount_to_shares(shares_pool, coins_amount)
        }
    }

    public(friend) fun redeem_active_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        if (coins_amount == 0) return 0;
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        let shares_to_redeem = amount_to_shares_to_redeem(&pool.active_shares, shareholder, coins_amount);
        pool_u64::redeem_shares(&mut pool.active_shares, shareholder, shares_to_redeem)
    }

    public(friend) fun redeem_inactive_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
        lockup_epoch: u64,
    ): u64 acquires DelegationPool {
        if (coins_amount == 0) return 0;
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        let current_lockup_epoch = current_lockup_epoch_internal(pool);
        let inactive_shares = vector::borrow_mut(&mut pool.inactive_shares, lockup_epoch);

        let shares_to_redeem = amount_to_shares_to_redeem(inactive_shares, shareholder, coins_amount);
        let redeemed_coins = pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

        // if delegator reactivated entire pending_inactive stake or withdrawn entire past stake,
        // enable unlocking operation again
        if (pool_u64::shares(inactive_shares, shareholder) == 0) {
            table::remove(&mut pool.pending_withdrawal, shareholder);
        };

        // if withdrawn the last shares from past pending_inactive shares pool, delete it
        if (lockup_epoch < current_lockup_epoch && pool_u64::total_coins(inactive_shares) == 0) {
            let inactive_shares = vector::remove<pool_u64::Pool>(&mut pool.inactive_shares, lockup_epoch);
            pool_u64::destroy_empty(inactive_shares);
        };

        redeemed_coins
    }

    public(friend) fun end_lockup_epoch(pool_address: address): bool acquires DelegationPool {
        if (!delegation_pool_exists(pool_address)) {
            return false
        };
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        // if no pending_inactive stake on the lockup epoch to be ended, reuse its shares pool
        if (pool_u64::total_coins(latest_inactive_shares_pool(pool)) > 0) {
            // start this new lockup epoch with a fresh shares pool
            vector::push_back(&mut pool.inactive_shares, pool_u64::create());
        };
        true
    }

    public(friend) fun commit_epoch_rewards(
        pool_address: address,
        rewards_active: u64,
        rewards_pending_inactive: u64,
    ): bool acquires DelegationPool {
        if (!delegation_pool_exists(pool_address)) {
            return false
        };
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        // update total coins accumulated by active shares
        if (rewards_active > 0) {
            let total_coins_active = pool_u64::total_coins(&pool.active_shares);
            pool_u64::update_total_coins(&mut pool.active_shares, total_coins_active + rewards_active);
        };

        // update total coins accumulated by pending_inactive shares
        if (rewards_pending_inactive > 0) {
            let inactive_shares = latest_inactive_shares_pool(pool);
            let total_coins_pending_inactive = pool_u64::total_coins(inactive_shares);
            pool_u64::update_total_coins(inactive_shares, total_coins_pending_inactive + rewards_pending_inactive);
        };
        true
    }
}