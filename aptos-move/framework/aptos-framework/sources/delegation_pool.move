module aptos_framework::delegation_pool {
    use std::error;

    use aptos_std::math64::min;
    use aptos_std::pool_u64;
    use aptos_std::table::{Self, Table};

    use aptos_framework::account;

    friend aptos_framework::stake;
    friend aptos_framework::delegate;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 1;

    /// TODO: limit the min delegation or allow unbound shares pool in framework
    const DELEGATORS_LIMIT: u64 = 10_000;

    struct DelegationPool has key {
        active_shares: pool_u64::Pool,
        inactive_shares: Table<u64, pool_u64::Pool>,
        lockup_epoch: u64,
        stake_pool_signer_cap: account::SignerCapability,
    }

    public(friend) fun initialize(stake_pool_signer: &signer, stake_pool_signer_cap: account::SignerCapability) {
        let inactive_shares = table::new<u64, pool_u64::Pool>();
        table::add(&mut inactive_shares, 1, pool_u64::create(DELEGATORS_LIMIT));

        move_to(stake_pool_signer, DelegationPool {
            lockup_epoch: 1,
            active_shares: pool_u64::create(DELEGATORS_LIMIT),
            inactive_shares,
            stake_pool_signer_cap,
        });
    }

    public(friend) fun get_stake_pool_signer(pool_address: address): signer acquires DelegationPool {
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
        borrow_global<DelegationPool>(pool_address).lockup_epoch
    }

    public(friend) fun buy_in_active_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        pool_u64::buy_in(&mut pool.active_shares, shareholder, coins_amount)
    }

    public(friend) fun buy_in_inactive_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        // cannot buy inactive shares, only pending inactive of current lockup epoch's pool
        pool_u64::buy_in(table::borrow_mut(&mut pool.inactive_shares, pool.lockup_epoch), shareholder, coins_amount)
    }

    public(friend) fun redeem_active_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
    ): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        coins_amount = min(coins_amount, pool_u64::balance(&pool.active_shares, shareholder));

        let shares_to_redeem = pool_u64::amount_to_shares(&pool.active_shares, coins_amount);
        pool_u64::redeem_shares(&mut pool.active_shares, shareholder, shares_to_redeem)
    }

    public(friend) fun redeem_inactive_shares(
        pool_address: address,
        shareholder: address,
        coins_amount: u64,
        lockup_epoch: u64,
    ): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let inactive_shares = table::borrow_mut(&mut pool.inactive_shares, lockup_epoch);

        coins_amount = min(coins_amount, pool_u64::balance(inactive_shares, shareholder));

        let shares_to_redeem = pool_u64::amount_to_shares(inactive_shares, coins_amount);
        let redeemed_coins = pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

        // if withdrawn the last shares from past pending_inactive shares pool, delete it
        if (lockup_epoch < pool.lockup_epoch && pool_u64::total_coins(inactive_shares) == 0) {
            let inactive_shares = table::remove<u64, pool_u64::Pool>(&mut pool.inactive_shares, lockup_epoch);
            pool_u64::destroy_empty(inactive_shares);
        };
        redeemed_coins
    }

    public(friend) fun end_lockup_epoch(pool_address: address): bool acquires DelegationPool {
        if (!delegation_pool_exists(pool_address)) {
            return false
        };
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        // if no pending_inactive stake on lockup epoch to end, reuse the shares pool
        if (pool_u64::total_coins(table::borrow(&pool.inactive_shares, pool.lockup_epoch)) == 0) {
            return true
        };

        // advance lookup epoch
        spec {
            assume pool.lockup_epoch + 1 <= MAX_U64;
        };
        pool.lockup_epoch = pool.lockup_epoch + 1;
        // start this new lockup epoch with a fresh shares pool
        table::add(&mut pool.inactive_shares, pool.lockup_epoch, pool_u64::create(DELEGATORS_LIMIT));
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
        let total_coins_active = pool_u64::total_coins(&pool.active_shares);
        pool_u64::update_total_coins(&mut pool.active_shares, total_coins_active + rewards_active);

        // update total coins accumulated by pending_inactive shares
        let inactive_shares = table::borrow_mut(&mut pool.inactive_shares, pool.lockup_epoch);
        let total_coins_pending_inactive = pool_u64::total_coins(inactive_shares);
        pool_u64::update_total_coins(inactive_shares, total_coins_pending_inactive + rewards_pending_inactive);
        true
    }
}