/// Concentrated liquidity pools, shaped after the Cellana and LiquidSwap v3
/// venues a Panora route reaches for deep pairs.
///
/// A hop walks the initialized ticks in front of the cursor, taking each one's
/// liquidity at its own price. How far it walks is what makes the read set
/// depend on trade size rather than on a constant.
module bench::dexr_pool_clmm {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::FungibleStore;
    use aptos_framework::object::Object;
    use bench::dexr_assets;
    use bench::dexr_math;

    /// Only the package address may configure pools.
    const E_NOT_BENCH: u64 = 1;

    /// Vault seed prefix, unique across the four backends.
    const PREFIX: vector<u8> = b"dexr_clmm";

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    /// Price denominator, and the most one tick's price may move off the
    /// previous one.
    const PRICE_DEN: u64 = 1_000_000;
    const PRICE_STEP: u64 = 5_000;

    /// Ceiling on a seeded tick's liquidity, so doubling it while jittering
    /// stays inside `u64`.
    const MAX_TICK_LIQUIDITY: u64 = 1_000_000_000_000_000;

    struct Tick has store, drop {
        liquidity: u64,
        price_num: u64,
        price_den: u64,
    }

    struct Pool has store {
        x: address,
        y: address,
        store_x: Object<FungibleStore>,
        store_y: Object<FungibleStore>,
        reserve_x: u64,
        reserve_y: u64,
        fee_bps: u64,
        ticks: vector<Tick>,
        /// First tick still holding liquidity.
        cursor: u64,
        fee_growth: u128,
        tick_liquidity: u64,
        tick_seed: u64,
    }

    struct Pools has key {
        pools: Table<u64, Pool>,
        n_pools: u64,
    }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Pools>(@bench)) {
            move_to(admin, Pools { pools: table::new(), n_pools: 0 });
        };
    }

    public entry fun create_pool(
        admin: &signer,
        x: address,
        y: address,
        reserve: u64,
        fee_bps: u64,
        n_ticks: u64,
        tick_liquidity: u64,
        tick_seed: u64,
    ) acquires Pools {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let pools = borrow_global_mut<Pools>(@bench);
        let pool_id = pools.n_pools;
        let store_x =
            dexr_assets::create_vault(admin, x, PREFIX, pool_id, 0, reserve);
        let store_y =
            dexr_assets::create_vault(admin, y, PREFIX, pool_id, 1, reserve);
        table::add(
            &mut pools.pools,
            pool_id,
            Pool {
                x,
                y,
                store_x,
                store_y,
                reserve_x: reserve,
                reserve_y: reserve,
                fee_bps,
                ticks: build_ticks(n_ticks, tick_liquidity, tick_seed),
                cursor: 0,
                fee_growth: 0,
                tick_liquidity,
                tick_seed,
            },
        );
        pools.n_pools = pool_id + 1;
    }

    /// Add `count` pools over consecutive entries of `assets`. Called in
    /// chunks, because creating a whole backend's pools in one transaction
    /// runs past the per-transaction execution limit.
    public entry fun seed_pools(
        admin: &signer,
        assets: vector<address>,
        count: u64,
        reserve: u64,
        fee_bps: u64,
        n_ticks: u64,
        tick_liquidity: u64,
        seed: u64,
    ) acquires Pools {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let n_assets = vector::length(&assets);
        if (n_assets < 2 || reserve == 0) {
            return
        };
        let state = seed;
        let i = 0;
        while (i < count) {
            state = lcg_next(state);
            let x = *vector::borrow(&assets, state % n_assets);
            let y = *vector::borrow(&assets, (state + 3) % n_assets);
            create_pool(
                admin,
                x,
                y,
                reserve + state % (reserve / 4 + 1),
                fee_bps,
                n_ticks,
                tick_liquidity,
                state,
            );
            i = i + 1;
        };
    }

    /// One step of the linear congruential generator. Reducing the state
    /// before the multiply keeps a seed wider than the modulus from
    /// overflowing `u64`.
    fun lcg_next(state: u64): u64 {
        ((state % LCG_MOD) * LCG_MUL + LCG_INC) % LCG_MOD
    }

    /// The initialized ticks a pool starts with, and the ones it goes back to
    /// once the cursor has walked off the end. Each tick prices strictly worse
    /// than the one before, so a trade that crosses more of them fills worse.
    fun build_ticks(n_ticks: u64, tick_liquidity: u64, seed: u64): vector<Tick> {
        let ticks = vector::empty<Tick>();
        let liquidity =
            if (tick_liquidity == 0) { 1 }
            else { dexr_math::min(tick_liquidity, MAX_TICK_LIQUIDITY) };
        let state = seed;
        let price_den = PRICE_DEN;
        let i = 0;
        while (i < n_ticks) {
            state = lcg_next(state);
            price_den = price_den + 1 + state % PRICE_STEP;
            vector::push_back(
                &mut ticks,
                Tick {
                    liquidity: liquidity + state % liquidity,
                    price_num: PRICE_DEN,
                    price_den,
                },
            );
            i = i + 1;
        };
        ticks
    }

    public fun n_pools(): u64 acquires Pools {
        if (!exists<Pools>(@bench)) { 0 } else { borrow_global<Pools>(@bench).n_pools }
    }

    public fun reserves(pool_id: u64): (u64, u64) acquires Pools {
        if (!exists<Pools>(@bench)) {
            return (0, 0)
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return (0, 0)
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        (pool.reserve_x, pool.reserve_y)
    }

    /// Ticks still holding liquidity in front of the cursor.
    public fun live_ticks(pool_id: u64): u64 acquires Pools {
        if (!exists<Pools>(@bench)) {
            return 0
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return 0
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        vector::length(&pool.ticks) - pool.cursor
    }

    /// Liquidity, price numerator and price denominator of tick `i`. An index
    /// past the end reads as an empty tick priced at one.
    public fun tick_at(pool_id: u64, i: u64): (u64, u64, u64) acquires Pools {
        if (!exists<Pools>(@bench)) {
            return (0, 1, 1)
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return (0, 1, 1)
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        if (i >= vector::length(&pool.ticks)) {
            return (0, 1, 1)
        };
        let tick = vector::borrow(&pool.ticks, i);
        (tick.liquidity, tick.price_num, tick.price_den)
    }

    /// Direction of a hop through `pool`. Whichever side the requested input
    /// sits on wins; a marker the pool holds on neither side takes the `x`
    /// side, unless the requested output is `x`.
    fun forward(pool: &Pool, asset_in: address, asset_out: address): bool {
        if (pool.y == asset_in) { false } else { pool.x != asset_out }
    }

    public fun swap(
        user: &signer,
        pool_id: u64,
        asset_in: address,
        asset_out: address,
        amount_in: u64,
    ): u64 acquires Pools {
        if (!exists<Pools>(@bench)) {
            return 0
        };
        let pools = borrow_global_mut<Pools>(@bench);
        if (pools.n_pools == 0) {
            return 0
        };
        let pool = table::borrow_mut(&mut pools.pools, pool_id % pools.n_pools);
        let forward = forward(pool, asset_in, asset_out);
        let (reserve_in, reserve_out) =
            if (forward) { (pool.reserve_x, pool.reserve_y) }
            else { (pool.reserve_y, pool.reserve_x) };
        let budget = dexr_math::clamp_in(amount_in, reserve_in);
        if (budget == 0 || reserve_out == 0) {
            return 0
        };

        let n_ticks = vector::length(&pool.ticks);
        let cursor = pool.cursor;
        let remaining = budget;
        let amount_out = 0;
        // The walk stops at the last initialized tick rather than inventing
        // liquidity, so a trade wider than the seeded range fills partially.
        while (remaining > 0 && cursor < n_ticks) {
            let tick = vector::borrow_mut(&mut pool.ticks, cursor);
            let step = dexr_math::min(remaining, tick.liquidity);
            let step_out = dexr_math::mul_div(step, tick.price_num, tick.price_den);
            tick.liquidity = tick.liquidity - step;
            let exhausted = tick.liquidity == 0;
            amount_out = amount_out + step_out;
            remaining = remaining - step;
            if (exhausted) {
                cursor = cursor + 1;
            };
        };
        // Refilling on wrap puts the initialized range back in front of the
        // next hop, so the pool never settles into quoting zero.
        if (cursor >= n_ticks) {
            pool.ticks = build_ticks(n_ticks, pool.tick_liquidity, pool.tick_seed);
            pool.cursor = 0;
        } else {
            pool.cursor = cursor;
        };

        let consumed = budget - remaining;
        if (amount_out >= reserve_out) {
            amount_out = reserve_out - 1;
        };
        if (forward) {
            pool.reserve_x = reserve_in + consumed;
            pool.reserve_y = reserve_out - amount_out;
        } else {
            pool.reserve_y = reserve_in + consumed;
            pool.reserve_x = reserve_out - amount_out;
        };
        pool.fee_growth = pool.fee_growth
            + (dexr_math::mul_div(consumed, pool.fee_bps, dexr_math::bps()) as u128);
        let (in_asset, in_store, out_asset, out_store) =
            if (forward) { (pool.x, pool.store_x, pool.y, pool.store_y) }
            else { (pool.y, pool.store_y, pool.x, pool.store_x) };
        dexr_assets::settle(
            user, in_asset, in_store, consumed, out_asset, out_store, amount_out
        );
        amount_out
    }

    /// Price a hop without taking it. The walk mirrors [`swap`] step for step,
    /// so a quote reports the fill the same trade would produce.
    public fun quote(
        pool_id: u64, asset_in: address, asset_out: address, amount_in: u64
    ): u64 acquires Pools {
        if (!exists<Pools>(@bench)) {
            return 0
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return 0
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        let (reserve_in, reserve_out) =
            if (forward(pool, asset_in, asset_out)) { (pool.reserve_x, pool.reserve_y) }
            else { (pool.reserve_y, pool.reserve_x) };
        let remaining = dexr_math::clamp_in(amount_in, reserve_in);
        let n_ticks = vector::length(&pool.ticks);
        let cursor = pool.cursor;
        let amount_out = 0;
        while (remaining > 0 && cursor < n_ticks) {
            let tick = vector::borrow(&pool.ticks, cursor);
            let step = dexr_math::min(remaining, tick.liquidity);
            amount_out = amount_out
                + dexr_math::mul_div(step, tick.price_num, tick.price_den);
            remaining = remaining - step;
            if (step == tick.liquidity) {
                cursor = cursor + 1;
            };
        };
        if (reserve_out == 0) { 0 }
        else if (amount_out >= reserve_out) { reserve_out - 1 }
        else { amount_out }
    }

    /// Mint `amount` into both vaults and put the initialized ticks back, so
    /// no pool drifts to a state where every trade clamps to a trivial amount.
    public fun rebalance(pool_id: u64, amount: u64) acquires Pools {
        if (!exists<Pools>(@bench)) {
            return
        };
        let pools = borrow_global_mut<Pools>(@bench);
        if (pools.n_pools == 0) {
            return
        };
        let pool = table::borrow_mut(&mut pools.pools, pool_id % pools.n_pools);
        let top_up = dexr_math::min(
            amount,
            dexr_math::min(
                dexr_math::headroom(pool.reserve_x),
                dexr_math::headroom(pool.reserve_y),
            ),
        );
        dexr_assets::fund_vault(pool.x, pool.store_x, top_up);
        dexr_assets::fund_vault(pool.y, pool.store_y, top_up);
        pool.reserve_x = pool.reserve_x + top_up;
        pool.reserve_y = pool.reserve_y + top_up;
        pool.ticks = build_ticks(
            vector::length(&pool.ticks), pool.tick_liquidity, pool.tick_seed
        );
        pool.cursor = 0;
    }
}
