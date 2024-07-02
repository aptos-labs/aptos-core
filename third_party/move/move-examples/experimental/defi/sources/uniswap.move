// This module defines a simple dex pool based on Uniswap V1
// that supports swapping coins and adding and removing liquidity.
module defi::uniswap {
    const PoolAddr: address = @defi;

    struct Pool has key {
        coin1: u64,
        coin2: u64,
        total_share: u64
    }

    // This swapping function is based on the formula from the Uniswap v1 codebase.
    public fun coin1_to_coin2_swap_code(coin1_in: u64): u64 acquires Pool {
        let pool = borrow_global_mut<Pool>(PoolAddr);
        let coin2_out = getInputPrice(coin1_in, pool.coin1, pool.coin2);
        pool.coin1 = pool.coin1 + coin1_in;
        pool.coin2 = pool.coin2 - coin2_out;
        coin2_out
    }
    spec coin1_to_coin2_swap_code {
        let old_pool = global<Pool>(PoolAddr);
        let post new_pool = global<Pool>(PoolAddr);
        ensures old_pool.coin1 * old_pool.coin2 <= new_pool.coin1 * new_pool.coin2; // safety property
    }
    fun getInputPrice(input_amount: u64, input_reserve: u64, output_reserve: u64): u64 {
        let input_amount_with_fee = input_amount * 997;
        let numerator = input_amount_with_fee * output_reserve;
        let denominator = (input_reserve * 1000) + input_amount_with_fee;
        numerator / denominator
    }

    // This swapping function simplifies the function `coin1_to_coin2_swap_code`.
    public fun coin1_to_coin2_swap_code_simple(coin1_in: u64): u64 acquires Pool {
        let pool = borrow_global_mut<Pool>(PoolAddr);
        let coin2_out = (997 * coin1_in * pool.coin2) / (1000 * pool.coin1 + 997 * coin1_in);
        pool.coin1 = pool.coin1 + coin1_in;
        pool.coin2 = pool.coin2 - coin2_out;
        coin2_out
    }
    spec coin1_to_coin2_swap_code_simple {
        let old_pool = global<Pool>(PoolAddr);
        let post new_pool = global<Pool>(PoolAddr);
        ensures old_pool.coin1 * old_pool.coin2 <= new_pool.coin1 * new_pool.coin2; // safety property
    }

    // This swapping function is based on the whitepaper that can be found at:
    // https://hackmd.io/@HaydenAdams/HJ9jLsfTz?type=view#ETH-%E2%87%84-ERC20-Trades
    // This version of the swapping function is not safe because it does not satisfy the safety property,
    // having an unbounded rounding error.
    public fun coin1_to_coin2_swap_whitepaper(coin1_in: u64): u64 acquires Pool {
        assert! (coin1_in > 0, 1);
        let fee = coin1_in * 3 / 1000; // 0.3%
        let pool = borrow_global_mut<Pool>(PoolAddr);
        spec {
            assume pool.coin1 > 0;
            assume pool.coin2 > 0;
        };
        let inv = pool.coin1 * pool.coin2; // `inv` may need to be u128
        let new_pool_coin1 = pool.coin1 + coin1_in;
        let new_pool_coin2 = inv / (new_pool_coin1 - fee); // No div-by-zero because (new_pool_coin1 - fee) cannot be 0.
        let coin2_out = pool.coin2 - new_pool_coin2;
        pool.coin1 = new_pool_coin1;
        pool.coin2 = new_pool_coin2;
        coin2_out
    }
    spec coin1_to_coin2_swap_whitepaper {
        let old_pool = global<Pool>(PoolAddr);
        let post new_pool = global<Pool>(PoolAddr);
        ensures old_pool.coin1 < new_pool.coin1;
        ensures old_pool.coin2 >= new_pool.coin2;
        ensures old_pool.coin1 * old_pool.coin2 <= new_pool.coin1 * new_pool.coin2; // safety property
    }

    // Adds liquidity to the pool and returns the liquidity share.
    public fun add_liquidity(coin1_in: u64, coin2_in: u64): u64 // returns liquidity share
    acquires Pool {
        let pool = borrow_global_mut<Pool>(PoolAddr);
        spec {
            assume pool.coin1 > 0;
            assume pool.coin2 > 0;
        };

        let coin1_added = coin1_in;
        let share_minted = (coin1_added * pool.total_share) / pool.coin1;
        let coin2_added = (share_minted * pool.coin2) / pool.total_share;
        // An alternative definition of coin2_added is as follows:
        // let coin2_added = (coin1_added  * pool.coin2 ) / pool.coin1;

        assert!(coin2_in == coin2_added, 1);

        pool.coin1 = pool.coin1 + coin1_added;
        pool.coin2 = pool.coin2 + coin2_added;
        pool.total_share = pool.total_share + share_minted;

        share_minted
    }
    spec add_liquidity {
        let old_pool = global<Pool>(PoolAddr);
        let post new_pool = global<Pool>(PoolAddr);
        ensures old_pool.coin1 <= new_pool.coin1;
        ensures old_pool.coin2 <= new_pool.coin2;
        ensures old_pool.total_share <= new_pool.total_share;
    }

    // Removes liquidity from the pool and returns the coins.
    public fun remove_liquidity(share: u64): (u64, u64) // returns (coin1, coin2)
    acquires Pool {
        let pool = borrow_global_mut<Pool>(PoolAddr);

        assert!(share <= pool.total_share, 1);

        let coin1_removed = (pool.coin1 * share) / pool.total_share;
        let coin2_removed = (pool.coin2 * share) / pool.total_share;

        pool.coin1 = pool.coin1 - coin1_removed;
        pool.coin2 = pool.coin2 - coin2_removed;
        pool.total_share = pool.total_share - share;

        (coin1_removed, coin2_removed)
    }
    spec remove_liquidity {
        let old_pool = global<Pool>(PoolAddr);
        let post new_pool = global<Pool>(PoolAddr);
        ensures old_pool.coin1 >= new_pool.coin1;
        ensures old_pool.coin2 >= new_pool.coin2;
        ensures old_pool.total_share >= new_pool.total_share;
    }

    #[verify_only]
    // The "No Free Money Theorem" states that if you add liquidity to a pool, then remove liquidity from the pool,
    // you will not end up with more money than you started with.
    fun no_free_money_theorem(coin1_in: u64, coin2_in: u64): (u64, u64) acquires Pool {
        let share = add_liquidity(coin1_in, coin2_in);
        remove_liquidity(share)
    }
    spec no_free_money_theorem {
        ensures result_1 <= coin1_in;
        ensures result_2 <= coin2_in;
    }
}
