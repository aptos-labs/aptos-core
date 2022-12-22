module aptos_framework::delegation_pool {
    use std::error;
    use std::vector;

    use aptos_std::pool_u64_unbound as pool_u64;
    use aptos_std::table::{Self, Table};

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::reconfiguration::{last_reconfiguration_time};
    use aptos_framework::stake;

    friend aptos_framework::delegate;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 1;

    /// There is a pending withdrawal to be executed before unlocking any stake
    const EPENDING_WITHDRAWAL_EXISTS: u64 = 2;

    /// Conversion factor between seconds and microseconds
    const MICRO_CONVERSION_FACTOR: u64 = 1000000;

    struct DelegationPool has key {
        active_shares: pool_u64::Pool,
        inactive_shares: vector<pool_u64::Pool>,
        pending_withdrawal: Table<address, u64>,
        stake_pool_signer_cap: account::SignerCapability,

        // Current lockup cycle's known expiration timestamp (provided `increase_lockup` is never called)
        locked_until_secs: u64,
        // Total (inactive) coins on the share pools over all ended lockup epochs
        total_coins_inactive: u64,

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
            locked_until_secs: 0,
            total_coins_inactive: 0,
            add_stake_events: account::new_event_handle<AddStakeEvent>(stake_pool_signer),
            reactivate_stake_events: account::new_event_handle<ReactivateStakeEvent>(stake_pool_signer),
            unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(stake_pool_signer),
            withdraw_stake_events: account::new_event_handle<WithdrawStakeEvent>(stake_pool_signer),
        });
    }

    public(friend) fun get_stake_pool_signer(pool_address: address): signer acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        // refresh total coins on share pools and attempt to advance lockup epoch at each user operation
        commit_epoch_rewards(pool_address);
        account::create_signer_with_capability(&borrow_global<DelegationPool>(pool_address).stake_pool_signer_cap)
    }

    public fun delegation_pool_exists(addr: address): bool {
        exists<DelegationPool>(addr)
    }

    /// there are stake pools proxied by no delegation pool
    public fun assert_delegation_pool_exists(pool_address: address) {
        assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));
    }

    public fun get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) acquires DelegationPool {
        let (active, inactive, pending_active, pending_inactive) = stake::get_stake(pool_address);
        let (withdrawal_exists, withdrawal_lockup_epoch) = pending_withdrawal_exists(pool_address, delegator_address);

        let pool = borrow_global<DelegationPool>(pool_address);
        active = pool_u64::shares_to_amount_with_total_coins(
            &pool.active_shares,
            pool_u64::shares(&pool.active_shares, delegator_address),
            active + pending_active
        );

        (inactive, pending_inactive) = if (withdrawal_exists) {
            let current_lockup_epoch = current_lockup_epoch_internal(pool);
            if (withdrawal_lockup_epoch < current_lockup_epoch) {
                (
                    pool_u64::balance(
                        vector::borrow(&pool.inactive_shares, withdrawal_lockup_epoch),
                        delegator_address
                    ),
                    0
                )
            } else {
                let pending_or_inactive_pool = vector::borrow(&pool.inactive_shares, current_lockup_epoch);
                let pending_or_inactive_shares = pool_u64::shares(pending_or_inactive_pool, delegator_address);
                if (last_reconfiguration_time() / MICRO_CONVERSION_FACTOR >= pool.locked_until_secs) {
                    (
                        pool_u64::shares_to_amount_with_total_coins(
                            pending_or_inactive_pool,
                            pending_or_inactive_shares,
                            inactive - pool.total_coins_inactive
                        ),
                        0
                    )
                } else {
                    (
                        0,
                        pool_u64::shares_to_amount_with_total_coins(
                            pending_or_inactive_pool,
                            pending_or_inactive_shares,
                            pending_inactive
                        )
                    )
                }
            }
        } else { (0, 0) };
        (active, inactive, pending_inactive)
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
        let pool = borrow_global<DelegationPool>(pool_address);
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

        if (lockup_epoch < current_lockup_epoch) {
            // withdrawing from ended lockup epoch requires total inactive coins to be decreased
            pool.total_coins_inactive = pool.total_coins_inactive - redeemed_coins;

            // delete shares pool of ended lockup epoch if everyone have withdrawn
            if (pool_u64::total_coins(inactive_shares) == 0) {
                pool_u64::destroy_empty(vector::remove<pool_u64::Pool>(&mut pool.inactive_shares, lockup_epoch));
            }
        };

        redeemed_coins
    }

    fun commit_epoch_rewards(pool_address: address) acquires DelegationPool {
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let (active, inactive, pending_active, pending_inactive) = stake::get_stake(pool_address);

        // update total coins accumulated by `active` + `pending_active` shares
        pool_u64::update_total_coins(&mut pool.active_shares, active + pending_active);

        // advance lockup epoch on delegation pool if it already passed on the stake one
        if (last_reconfiguration_time() / MICRO_CONVERSION_FACTOR >= pool.locked_until_secs) {
            pool.locked_until_secs = stake::get_lockup_secs(pool_address);

            // `inactive` on stake pool == remaining inactive coins over ended lockup epochs +
            // `pending_inactive` stake and its rewards (both inactivated) on this ending lockup
            let ended_lockup_total_coins = inactive - pool.total_coins_inactive;
            pool_u64::update_total_coins(latest_inactive_shares_pool(pool), ended_lockup_total_coins);

            // capture inactive coins over all ended lockup cycles (including this ending one)
            pool.total_coins_inactive = inactive;

            // if no `pending_inactive` stake on the ending lockup, reuse its shares pool
            if (ended_lockup_total_coins > 0) {
                // start new lockup epoch with empty shares pool
                vector::push_back(&mut pool.inactive_shares, pool_u64::create());
            }
        } else {
            // update total coins accumulated by `pending_inactive` shares during this live lockup
            pool_u64::update_total_coins(latest_inactive_shares_pool(pool), pending_inactive)
        }
    }
}