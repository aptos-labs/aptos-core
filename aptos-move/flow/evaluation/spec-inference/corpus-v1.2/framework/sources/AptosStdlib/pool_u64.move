///
/// Simple module for tracking and calculating shares of a pool of coins. The shares are worth more as the total coins in
/// the pool increases. New shareholder can buy more shares or redeem their existing shares.
///
/// Example flow:
/// 1. Pool start outs empty.
/// 2. Shareholder A buys in with 1000 coins. A will receive 1000 shares in the pool. Pool now has 1000 total coins and
/// 1000 total shares.
/// 3. Pool appreciates in value from rewards and now has 2000 coins. A's 1000 shares are now worth 2000 coins.
/// 4. Shareholder B now buys in with 1000 coins. Since before the buy in, each existing share is worth 2 coins, B will
/// receive 500 shares in exchange for 1000 coins. Pool now has 1500 shares and 3000 coins.
/// 5. Pool appreciates in value from rewards and now has 6000 coins.
/// 6. A redeems 500 shares. Each share is worth 6000 / 1500 = 4. A receives 2000 coins. Pool has 4000 coins and 1000
/// shares left.
///
module aptos_std::pool_u64 {
    use aptos_std::simple_map::{Self, SimpleMap};
    use std::error;
    use std::vector;
    use aptos_std::math64;

    /// Shareholder not present in pool.
    const ESHAREHOLDER_NOT_FOUND: u64 = 1;
    /// There are too many shareholders in the pool.
    const ETOO_MANY_SHAREHOLDERS: u64 = 2;
    /// Cannot destroy non-empty pool.
    const EPOOL_IS_NOT_EMPTY: u64 = 3;
    /// Cannot redeem more shares than the shareholder has in the pool.
    const EINSUFFICIENT_SHARES: u64 = 4;
    /// Shareholder cannot have more than u64.max shares.
    const ESHAREHOLDER_SHARES_OVERFLOW: u64 = 5;
    /// Pool's total coins cannot exceed u64.max.
    const EPOOL_TOTAL_COINS_OVERFLOW: u64 = 6;
    /// Pool's total shares cannot exceed u64.max.
    const EPOOL_TOTAL_SHARES_OVERFLOW: u64 = 7;

    const MAX_U64: u64 = 18446744073709551615;

    struct Pool has store {
        shareholders_limit: u64,
        total_coins: u64,
        total_shares: u64,
        shares: SimpleMap<address, u64>,
        shareholders: vector<address>,
        // Default to 1. This can be used to minimize rounding errors when computing shares and coins amount.
        // However, users need to make sure the coins amount don't overflow when multiplied by the scaling factor.
        scaling_factor: u64,
    }

    /// Create a new pool.
    public fun new(shareholders_limit: u64): Pool {
        // Default to a scaling factor of 1 (effectively no scaling).
        create_with_scaling_factor(shareholders_limit, 1)
    }

    #[deprecated]
    /// Deprecated. Use `new` instead.
    /// Create a new pool.
    public fun create(shareholders_limit: u64): Pool {
        new(shareholders_limit)
    }

    /// Create a new pool with custom `scaling_factor`.
    public fun create_with_scaling_factor(shareholders_limit: u64, scaling_factor: u64): Pool {
        Pool {
            shareholders_limit,
            total_coins: 0,
            total_shares: 0,
            shares: simple_map::create<address, u64>(),
            shareholders: vector::empty<address>(),
            scaling_factor,
        }
    }

    /// Destroy an empty pool. This will fail if the pool has any balance of coins.
    public fun destroy_empty(self: Pool) {
        assert!(self.total_coins == 0, error::invalid_state(EPOOL_IS_NOT_EMPTY));
        let Pool {
            shareholders_limit: _,
            total_coins: _,
            total_shares: _,
            shares: _,
            shareholders: _,
            scaling_factor: _,
        } = self;
    }

    /// Return `self`'s total balance of coins.
    public fun total_coins(self: &Pool): u64 {
        self.total_coins
    }

    /// Return the total number of shares across all shareholders in `self`.
    public fun total_shares(self: &Pool): u64 {
        self.total_shares
    }

    /// Return true if `shareholder` is in `self`.
    public fun contains(self: &Pool, shareholder: address): bool {
        self.shares.contains_key(&shareholder)
    }

    /// Return the number of shares of `stakeholder` in `self`.
    public fun shares(self: &Pool, shareholder: address): u64 {
        if (self.contains(shareholder)) {
            *self.shares.borrow(&shareholder)
        } else {
            0
        }
    }

    /// Return the balance in coins of `shareholder` in `self`.
    public fun balance(self: &Pool, shareholder: address): u64 {
        let num_shares = self.shares(shareholder);
        self.shares_to_amount(num_shares)
    }

    /// Return the list of shareholders in `self`.
    public fun shareholders(self: &Pool): vector<address> {
        self.shareholders
    }

    /// Return the number of shareholders in `self`.
    public fun shareholders_count(self: &Pool): u64 {
        self.shareholders.length()
    }

    /// Update `self`'s total balance of coins.
    public fun update_total_coins(self: &mut Pool, new_total_coins: u64) {
        self.total_coins = new_total_coins;
    }

    /// Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.
    public fun buy_in(self: &mut Pool, shareholder: address, coins_amount: u64): u64 {
        if (coins_amount == 0) return 0;

        let new_shares = self.amount_to_shares(coins_amount);
        assert!(MAX_U64 - self.total_coins >= coins_amount, error::invalid_argument(EPOOL_TOTAL_COINS_OVERFLOW));
        assert!(MAX_U64 - self.total_shares >= new_shares, error::invalid_argument(EPOOL_TOTAL_COINS_OVERFLOW));

        self.total_coins += coins_amount;
        self.total_shares += new_shares;
        self.add_shares(shareholder, new_shares);
        new_shares
    }

    /// Add the number of shares directly for `shareholder` in `self`.
    /// This would dilute other shareholders if the pool's balance of coins didn't change.
    fun add_shares(self: &mut Pool, shareholder: address, new_shares: u64): u64 {
        if (self.contains(shareholder)) {
            let existing_shares = self.shares.borrow_mut(&shareholder);
            let current_shares = *existing_shares;
            assert!(MAX_U64 - current_shares >= new_shares, error::invalid_argument(ESHAREHOLDER_SHARES_OVERFLOW));

            *existing_shares = current_shares + new_shares;
            *existing_shares
        } else if (new_shares > 0) {
            assert!(
                self.shareholders.length() < self.shareholders_limit,
                error::invalid_state(ETOO_MANY_SHAREHOLDERS),
            );

            self.shareholders.push_back(shareholder);
            self.shares.add(shareholder, new_shares);
            new_shares
        } else {
            new_shares
        }
    }

    /// Allow `shareholder` to redeem their shares in `self` for coins.
    public fun redeem_shares(self: &mut Pool, shareholder: address, shares_to_redeem: u64): u64 {
        assert!(self.contains(shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        assert!(self.shares(shareholder) >= shares_to_redeem, error::invalid_argument(EINSUFFICIENT_SHARES));

        if (shares_to_redeem == 0) return 0;

        let redeemed_coins = self.shares_to_amount(shares_to_redeem);
        self.total_coins -= redeemed_coins;
        self.total_shares -= shares_to_redeem;
        self.deduct_shares(shareholder, shares_to_redeem);

        redeemed_coins
    }

    /// Transfer shares from `shareholder_1` to `shareholder_2`.
    public fun transfer_shares(
        self: &mut Pool,
        shareholder_1: address,
        shareholder_2: address,
        shares_to_transfer: u64,
    ) {
        assert!(self.contains(shareholder_1), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        assert!(self.shares(shareholder_1) >= shares_to_transfer, error::invalid_argument(EINSUFFICIENT_SHARES));
        if (shares_to_transfer == 0) return;

        self.deduct_shares(shareholder_1, shares_to_transfer);
        self.add_shares(shareholder_2, shares_to_transfer);
    }

    /// Directly deduct `shareholder`'s number of shares in `self` and return the number of remaining shares.
    fun deduct_shares(self: &mut Pool, shareholder: address, num_shares: u64): u64 {
        assert!(self.contains(shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
        assert!(self.shares(shareholder) >= num_shares, error::invalid_argument(EINSUFFICIENT_SHARES));

        let existing_shares = self.shares.borrow_mut(&shareholder);
        *existing_shares -= num_shares;

        // Remove the shareholder completely if they have no shares left.
        let remaining_shares = *existing_shares;
        if (remaining_shares == 0) {
            let (_, shareholder_index) = self.shareholders.index_of(&shareholder);
            self.shareholders.remove(shareholder_index);
            self.shares.remove(&shareholder);
        };

        remaining_shares
    }

    /// Return the number of new shares `coins_amount` can buy in `self`.
    /// `amount` needs to big enough to avoid rounding number.
    public fun amount_to_shares(self: &Pool, coins_amount: u64): u64 {
        self.amount_to_shares_with_total_coins(coins_amount, self.total_coins)
    }

    /// Return the number of new shares `coins_amount` can buy in `self` with a custom total coins number.
    /// `amount` needs to big enough to avoid rounding number.
    public fun amount_to_shares_with_total_coins(self: &Pool, coins_amount: u64, total_coins: u64): u64 {
        // No shares yet so amount is worth the same number of shares.
        if (self.total_coins == 0 || self.total_shares == 0) {
            // Multiply by scaling factor to minimize rounding errors during internal calculations for buy ins/redeems.
            // This can overflow but scaling factor is expected to be chosen carefully so this would not overflow.
            coins_amount * self.scaling_factor
        } else {
            // Shares price = total_coins / total existing shares.
            // New number of shares = new_amount / shares_price = new_amount * existing_shares / total_amount.
            // We rearrange the calc and do multiplication first to avoid rounding errors.
            self.multiply_then_divide(coins_amount, self.total_shares, total_coins)
        }
    }

    /// Return the number of coins `shares` are worth in `self`.
    /// `shares` needs to big enough to avoid rounding number.
    public fun shares_to_amount(self: &Pool, shares: u64): u64 {
        self.shares_to_amount_with_total_coins(shares, self.total_coins)
    }

    /// Return the number of coins `shares` are worth in `self` with a custom total coins number.
    /// `shares` needs to big enough to avoid rounding number.
    public fun shares_to_amount_with_total_coins(self: &Pool, shares: u64, total_coins: u64): u64 {
        // No shares or coins yet so shares are worthless.
        if (self.total_coins == 0 || self.total_shares == 0) {
            0
        } else {
            // Shares price = total_coins / total existing shares.
            // Shares worth = shares * shares price = shares * total_coins / total existing shares.
            // We rearrange the calc and do multiplication first to avoid rounding errors.
            self.multiply_then_divide(shares, total_coins, self.total_shares)
        }
    }

    public fun multiply_then_divide(self: &Pool, x: u64, y: u64, z: u64): u64 {
        math64::mul_div(x, y, z)
    }

    #[test_only]
    public fun destroy_pool(self: Pool) {
        let Pool {
            shareholders_limit: _,
            total_coins: _,
            total_shares: _,
            shares: _,
            shareholders: _,
            scaling_factor: _,
        } = self;
    }

    #[test]
    public entry fun test_buy_in_and_redeem() {
        let pool = new(5);

        // Shareholders 1 and 2 buy in first.
        pool.buy_in(@1, 1000);
        pool.buy_in(@2, 2000);
        assert!(pool.shareholders_count() == 2, 0);
        assert!(pool.total_coins() == 3000, 1);
        assert!(pool.total_shares() == 3000, 2);
        assert!(pool.shares(@1) == 1000, 3);
        assert!(pool.shares(@2) == 2000, 4);
        assert!(pool.balance(@1) == 1000, 5);
        assert!(pool.balance(@2) == 2000, 6);

        // Pool increases in value.
        pool.update_total_coins(5000);
        assert!(pool.shares(@1) == 1000, 7);
        assert!(pool.shares(@2) == 2000, 8);
        let expected_balance_1 = 1000 * 5000 / 3000;
        assert!(pool.balance(@1) == expected_balance_1, 9);
        let expected_balance_2 = 2000 * 5000 / 3000;
        assert!(pool.balance(@2) == expected_balance_2, 10);

        // Shareholder 3 buys in into the 5000-coin pool with 1000 coins. There are 3000 existing shares.
        let expected_shares = 1000 * 3000 / 5000;
        pool.buy_in(@3, 1000);
        assert!(pool.shares(@3) == expected_shares, 11);
        assert!(pool.balance(@3) == 1000, 12);

        // Pool increases more in value.
        pool.update_total_coins(8000);

        // Shareholders 1 and 2 redeem.
        let all_shares = 3000 + expected_shares;
        assert!(pool.total_shares() == all_shares, 13);
        let expected_value_per_500_shares = 500 * 8000 / all_shares;
        assert!(pool.redeem_shares(@1, 500) == expected_value_per_500_shares, 14);
        assert!(pool.redeem_shares(@1, 500) == expected_value_per_500_shares, 15);
        assert!(pool.redeem_shares(@2, 2000) == expected_value_per_500_shares * 4, 16);

        // Due to a very small rounding error of 1, shareholder 3 actually has 1 more coin.
        let shareholder_3_balance = expected_value_per_500_shares * 6 / 5 + 1;
        assert!(pool.balance(@3) == shareholder_3_balance, 17);
        assert!(pool.total_coins() == shareholder_3_balance, 18);
        assert!(pool.shareholders_count() == 1, 19);
        let num_shares_3 = pool.shares(@3);
        assert!(pool.redeem_shares(@3, num_shares_3) == shareholder_3_balance, 20);

        // Nothing left.
        assert!(pool.shareholders_count() == 0, 21);
        pool.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 196611, location = Self)]
    public entry fun test_destroy_empty_should_fail_if_not_empty() {
        let pool = new(1);
        pool.update_total_coins(100);
        pool.destroy_empty();
    }

    #[test]
    public entry fun test_buy_in_and_redeem_large_numbers() {
        let pool = new(2);
        let half_max_u64 = MAX_U64 / 2;
        let shares_1 = pool.buy_in(@1, half_max_u64);
        assert!(shares_1 == half_max_u64, 0);
        let shares_2 = pool.buy_in(@2, half_max_u64 + 1);
        assert!(shares_2 == half_max_u64 + 1, 1);
        assert!(pool.total_shares() == MAX_U64, 2);
        assert!(pool.total_coins() == MAX_U64, 3);
        assert!(pool.redeem_shares(@1, shares_1) == half_max_u64, 4);
        assert!(pool.redeem_shares(@2, shares_2) == half_max_u64 + 1, 5);
        pool.destroy_empty();
    }

    #[test]
    public entry fun test_buy_in_and_redeem_large_numbers_with_scaling_factor() {
        let scaling_factor = 100;
        let pool = create_with_scaling_factor(2, scaling_factor);
        let coins_amount = MAX_U64 / 100;
        let shares = pool.buy_in(@1, coins_amount);
        assert!(pool.total_shares() == coins_amount * scaling_factor, 0);
        assert!(pool.total_coins() == coins_amount, 1);
        assert!(pool.redeem_shares(@1, shares) == coins_amount, 2);
        pool.destroy_empty();
    }

    #[test]
    public entry fun test_buy_in_zero_amount() {
        let pool = new(2);
        pool.buy_in(@1, 100);
        assert!(pool.buy_in(@2, 0) == 0, 0);
        assert!(pool.total_shares() == pool.shares(@1), 1);
        assert!(pool.shareholders_count() == 1, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_buy_in_with_small_coins_amount() {
        let pool = new(2);
        // Shareholder 1 buys in with 1e17 coins.
        pool.buy_in(@1, 100000000000000000);
        // Shareholder 2 buys in with a very small amount.
        assert!(pool.buy_in(@2, 1) == 1, 0);
        // Pool's total coins increases by 20%. Shareholder 2 shouldn't see any actual balance increase as it gets
        // rounded down.
        let total_coins = pool.total_coins();
        pool.update_total_coins(total_coins * 6 / 5);
        // Minus 1 due to rounding error.
        assert!(pool.balance(@1) == 100000000000000000 * 6 / 5 - 1, 1);
        assert!(pool.balance(@2) == 1, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_add_zero_shares_should_not_add_shareholder() {
        let pool = new(1);
        pool.update_total_coins(1000);
        assert!(pool.add_shares(@1, 0) == 0, 0);
        assert!(pool.shareholders_count() == 0, 1);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_add_zero_shares_returns_existing_number_of_shares() {
        let pool = new(1);
        pool.update_total_coins(1000);
        pool.add_shares(@1, 1);
        assert!(pool.shares(@1) == pool.add_shares(@1, 0), 0);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_add_shares_existing_shareholder() {
        let pool = new(1);
        pool.update_total_coins(1000);
        pool.add_shares(@1, 1);
        pool.add_shares(@1, 2);
        assert!(pool.shares(@1) == 3, 0);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_add_shares_new_shareholder() {
        let pool = new(2);
        pool.update_total_coins(1000);
        pool.add_shares(@1, 1);
        pool.add_shares(@2, 2);
        assert!(pool.shares(@1) == 1, 0);
        assert!(pool.shares(@2) == 2, 1);
        pool.destroy_pool();
    }

    #[test]
    #[expected_failure(abort_code = 196610, location = Self)]
    public entry fun test_add_shares_should_enforce_shareholder_limit() {
        let pool = new(2);
        pool.add_shares(@1, 1);
        pool.add_shares(@2, 2);
        pool.add_shares(@3, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_add_shares_should_work_after_reducing_shareholders_below_limit() {
        let pool = new(3);
        pool.add_shares(@1, 1);
        pool.add_shares(@2, 2);
        pool.deduct_shares(@2, 2);
        pool.add_shares(@3, 3);
        assert!(pool.shares(@3) == 3, 0);
        pool.destroy_pool();
    }

    #[test]
    #[expected_failure(abort_code = 65537, location = Self)]
    public entry fun test_redeem_shares_non_existent_shareholder() {
        let pool = new(1);
        pool.add_shares(@1, 1);
        pool.redeem_shares(@2, 1);
        pool.destroy_pool();
    }

    #[test]
    #[expected_failure(abort_code = 65540, location = Self)]
    public entry fun test_redeem_shares_insufficient_shares() {
        let pool = new(1);
        pool.add_shares(@1, 1);
        pool.redeem_shares(@1, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_redeem_small_number_of_shares() {
        let pool = new(2);
        // 1e17 shares and coins.
        pool.buy_in(@1, 100000000000000000);
        pool.buy_in(@2, 100000000000000000);
        assert!(pool.redeem_shares(@1, 1) == 1, 0);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_redeem_zero_shares() {
        let pool = new(2);
        pool.buy_in(@1, 1);
        assert!(pool.redeem_shares(@1, 0) == 0, 0);
        assert!(pool.shares(@1) == 1, 1);
        assert!(pool.total_coins() == 1, 2);
        assert!(pool.total_shares() == 1, 3);
        pool.destroy_pool();
    }

    #[test]
    #[expected_failure(abort_code = 65537, location = Self)]
    public entry fun test_deduct_shares_non_existent_shareholder() {
        let pool = new(1);
        pool.add_shares(@1, 1);
        pool.deduct_shares(@2, 1);
        pool.destroy_pool();
    }

    #[test]
    #[expected_failure(abort_code = 65540, location = Self)]
    public entry fun test_deduct_shares_insufficient_shares() {
        let pool = new(1);
        pool.add_shares(@1, 1);
        pool.deduct_shares(@1, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_deduct_shares_remove_shareholder_with_no_shares() {
        let pool = new(2);
        pool.add_shares(@1, 1);
        pool.add_shares(@2, 2);
        assert!(pool.shareholders_count() == 2, 0);
        pool.deduct_shares(@1, 1);
        assert!(pool.shareholders_count() == 1, 1);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_transfer_shares() {
        let pool = new(2);
        pool.add_shares(@1, 2);
        pool.add_shares(@2, 2);
        assert!(pool.shareholders_count() == 2, 0);
        pool.transfer_shares(@1, @2, 1);
        assert!(pool.shares(@1) == 1, 0);
        assert!(pool.shares(@2) == 3, 0);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_amount_to_shares_empty_pool() {
        let pool = new(1);
        // No total shares and total coins.
        assert!(pool.amount_to_shares(1000) == 1000, 0);

        // No total shares but some total coins.
        pool.update_total_coins(1000);
        assert!(pool.amount_to_shares(1000) == 1000, 1);

        // No total coins but some total shares.
        pool.update_total_coins(0);
        pool.add_shares(@1, 100);
        assert!(pool.amount_to_shares(1000) == 1000, 2);
        pool.destroy_pool();
    }

    #[test]
    public entry fun test_shares_to_amount_empty_pool() {
        let pool = new(1);
        // No total shares and total coins.
        assert!(pool.shares_to_amount(1000) == 0, 0);

        // No total shares but some total coins.
        pool.update_total_coins(1000);
        assert!(pool.shares_to_amount(1000) == 0, 1);

        // No total coins but some total shares.
        pool.update_total_coins(0);
        pool.add_shares(@1, 100);
        assert!(pool.shares_to_amount(1000) == 0, 2);
        pool.destroy_pool();
    }
}
