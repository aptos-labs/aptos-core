// Simple module for tracking and calculating shares of a pool of coins.
// This only supports amounts and shares < u64.max. Internally it stores everything as u128 to avoid overflow when
// doing u64 multiplication.
module aptos_std::pool_u64 {
    use aptos_std::simple_map::{Self, SimpleMap};
    use std::error;
    use std::vector;

    /// Shareholder not present in pool.
    const ESHAREHOLDER_NOT_FOUND: u64 = 1;
    /// There are too many shareholders in the pool.
    const ETOO_MANY_SHAREHOLDERS: u64 = 2;
    /// Cannot destroy non-empty pool.
    const EPOOL_IS_NOT_EMPTY: u64 = 3;
    /// Cannot redeem more shares than the shareholder has in the pool.
    const EINSUFFICIENT_SHARES: u64 = 4;

    struct Pool has store {
        shareholders_limit: u64,
        // We store numbers internally as u128 to make sure u64 multiplications don't overflow.
        total_coins: u128,
        total_shares: u128,
        shares: SimpleMap<address, u128>,
        shareholders: vector<address>,
    }

    /// Create a new pool.
    public fun create(shareholders_limit: u64): Pool {
        Pool {
            shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::create<address, u128>(),
            shareholders: vector::empty<address>(),
        }
    }

    /// Destroy an empty pool. This will fail if the pool has any balance of coins.
    public fun destroy_empty(pool: Pool) {
        assert!(pool.total_coins == 0, error::invalid_state(EPOOL_IS_NOT_EMPTY));
        let Pool {
            shareholders_limit: _,
            total_coins: _,
            total_shares: _,
            shares: _,
            shareholders: _,
        } = pool;
    }

    /// Return `pool`'s total balance of coins.
    public fun total_coins(pool: &Pool): u64 {
        (pool.total_coins as u64)
    }

    /// Return the total number of shares across all shareholders in `pool`.
    public fun total_shares(pool: &Pool): u64 {
        (pool.total_shares as u64)
    }

    /// Return true if `shareholder` is in `pool`.
    public fun contains(pool: &Pool, shareholder: address): bool {
        simple_map::contains_key(&pool.shares, &shareholder)
    }

    /// Return the number of shares of `stakeholder` in `pool`.
    public fun shares(pool: &Pool, shareholder: address): u64 {
        if (contains(pool, shareholder)) {
            (*simple_map::borrow(&pool.shares, &shareholder) as u64)
        } else {
            0
        }
    }

    /// Return the balance in coins of `shareholder` in `pool.`
    public fun balance(pool: &Pool, shareholder: address): u64 {
        let num_shares = shares(pool, shareholder);
        shares_to_amount(pool, num_shares)
    }

    /// Return the list of shareholders in `pool`.
    public fun shareholders(pool: &Pool): vector<address> {
        pool.shareholders
    }

    /// Return the number of shareholders in `pool`.
    public fun shareholders_count(pool: &Pool): u64 {
        vector::length(&pool.shareholders)
    }

    /// Update `pool`'s total balance of coins.
    public fun update_total_coins(pool: &mut Pool, new_total_coins: u64) {
        pool.total_coins = (new_total_coins as u128);
    }

    /// Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.
    public fun buy_in(pool: &mut Pool, shareholder: address, coins_amount: u64): u64 {
        let num_shares = amount_to_shares(pool, coins_amount);
        pool.total_coins = pool.total_coins + (coins_amount as u128);
        pool.total_shares = pool.total_shares + (num_shares as u128);
        add_shares(pool, shareholder, num_shares);

        num_shares
    }

    /// Add the number of shares directly for `shareholder` in `pool`.
    /// This would dilute other shareholders if the pool's balance of coins didn't change.
    public fun add_shares(pool: &mut Pool, shareholder: address, num_shares: u64): u64 {
        let num_shares_u128 = (num_shares as u128);
        if (contains(pool, shareholder)) {
            let existing_shares = simple_map::borrow_mut(&mut pool.shares, &shareholder);
            *existing_shares = *existing_shares + num_shares_u128;

            (*existing_shares as u64)
        } else {
            assert!(
                vector::length(&pool.shareholders) < pool.shareholders_limit,
                error::invalid_state(ETOO_MANY_SHAREHOLDERS),
            );

            vector::push_back(&mut pool.shareholders, shareholder);
            simple_map::add(&mut pool.shares, shareholder, num_shares_u128);

            num_shares
        }
    }

    /// Allow `shareholder` to redeem their shares in `pool` for coins.
    public fun redeem_shares(pool: &mut Pool, shareholder: address, num_shares: u64): u64 {
        assert!(contains(pool, shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        assert!(shares(pool, shareholder) >= num_shares, error::invalid_argument(EINSUFFICIENT_SHARES));

        let num_coins = shares_to_amount(pool, num_shares);
        pool.total_coins = pool.total_coins - (num_coins as u128);
        pool.total_shares = pool.total_shares - (num_shares as u128);
        deduct_shares(pool, shareholder, num_shares);

        num_coins
    }

    /// Directly deduct `shareholder`'s number of shares in `pool` and return the number of remaining shares.
    public fun deduct_shares(pool: &mut Pool, shareholder: address, num_shares: u64): u64 {
        assert!(contains(pool, shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        assert!(shares(pool, shareholder) >= num_shares, error::invalid_argument(EINSUFFICIENT_SHARES));

        let existing_shares = simple_map::borrow_mut(&mut pool.shares, &shareholder);
        *existing_shares = *existing_shares - (num_shares as u128);

        // Remove the shareholder completely if they have no shares left.
        let remaining_shares = (*existing_shares as u64);
        if (remaining_shares == 0) {
            let (_, shareholder_index) = vector::index_of(&pool.shareholders, &shareholder);
            vector::remove(&mut pool.shareholders, shareholder_index);
            simple_map::remove(&mut pool.shares, &shareholder);
        };

        remaining_shares
    }

    /// Return the number of new shares `coins_amount` can buy in `pool`.
    /// `amount` needs to big enough to avoid rounding number.
    public fun amount_to_shares(pool: &Pool, coins_amount: u64): u64 {
        // No shares yet so amount is worth the same number of shares.
        if (pool.total_shares == 0) {
            coins_amount
        } else {
            // Shares price = total_coins / total existing shares.
            // New number of shares = new_amount / shares_price = new_amount * existing_shares / total_amount.
            // We rearrange the calc and do multiplication first to avoid rounding errors.
            let num_shares_u128 = (coins_amount as u128) * pool.total_shares / pool.total_coins;
            (num_shares_u128 as u64)
        }
    }

    /// Return the number of coins `shares` are worth in `pool`.
    /// `shares` needs to big enough to avoid rounding number.
    public fun shares_to_amount(pool: &Pool, shares: u64): u64 {
        // No shares or coins yet so shares are worthless.
        if (pool.total_coins == 0 || pool.total_shares == 0) {
            0
        } else {
            // Shares price = total_coins / total existing shares.
            // Shares worth = shares * shares price = shares * total_coins / total existing shares.
            // We rearrange the calc and do multiplication first to avoid rounding errors.
            let num_coins_u128 = (shares as u128) * pool.total_coins / pool.total_shares;
            (num_coins_u128 as u64)
        }
    }
}
