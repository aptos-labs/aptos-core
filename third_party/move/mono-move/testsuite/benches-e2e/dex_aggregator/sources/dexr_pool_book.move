/// Order-book venues, shaped after the Econia-style markets a Panora route
/// falls back to when the pool depth is thin.
///
/// A hop fills from the front of the level vector, paying each level's own
/// price. Levels are sized in output units, so the cost of a hop tracks how
/// many of them the trade clears.
module bench::dexr_pool_book {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::FungibleStore;
    use aptos_framework::object::Object;
    use bench::dexr_assets;
    use bench::dexr_math;

    /// Only the package address may configure books.
    const E_NOT_BENCH: u64 = 1;

    /// Vault seed prefix, unique across the four backends.
    const PREFIX: vector<u8> = b"dexr_book";

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    /// Price denominator, and the most one level's price may move off the
    /// previous one.
    const PRICE_DEN: u64 = 1_000_000;
    const PRICE_STEP: u64 = 5_000;

    /// Ceiling on a seeded level's size, so doubling it while jittering stays
    /// inside `u64`.
    const MAX_LEVEL_SIZE: u64 = 1_000_000_000_000_000;

    struct Level has store, drop {
        /// Output units still resting at this price.
        size: u64,
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
        levels: vector<Level>,
        /// First level still resting.
        cursor: u64,
        filled_levels: u64,
        level_size: u64,
        level_seed: u64,
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
        depth: u64,
        level_size: u64,
        level_seed: u64,
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
                levels: build_levels(depth, level_size, level_seed),
                cursor: 0,
                filled_levels: 0,
                level_size,
                level_seed,
            },
        );
        pools.n_pools = pool_id + 1;
    }

    /// Add `count` books over consecutive entries of `assets`. Called in
    /// chunks, because creating a whole backend's books in one transaction
    /// runs past the per-transaction execution limit.
    public entry fun seed_pools(
        admin: &signer,
        assets: vector<address>,
        count: u64,
        reserve: u64,
        fee_bps: u64,
        depth: u64,
        level_size: u64,
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
            // Book `id` holds assets `id` and `id + 1`. A route picks the book
            // each of its hops lands on before it knows what that book holds,
            // so which pair a book holds has to follow from its id.
            let id = n_pools();
            let x = *vector::borrow(&assets, id % n_assets);
            let y = *vector::borrow(&assets, (id + 1) % n_assets);
            create_pool(
                admin,
                x,
                y,
                reserve + state % (reserve / 4 + 1),
                fee_bps,
                depth,
                level_size,
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

    /// The resting levels a book starts with, and the ones it goes back to
    /// once the cursor has walked off the end. Prices worsen with depth, so a
    /// trade that clears more levels gets a worse average fill.
    fun build_levels(depth: u64, level_size: u64, seed: u64): vector<Level> {
        let levels = vector::empty<Level>();
        let size =
            if (level_size == 0) { 1 }
            else { dexr_math::min(level_size, MAX_LEVEL_SIZE) };
        let state = seed;
        let price_den = PRICE_DEN;
        let i = 0;
        while (i < depth) {
            state = lcg_next(state);
            price_den = price_den + 1 + state % PRICE_STEP;
            vector::push_back(
                &mut levels,
                Level {
                    size: size + state % size,
                    price_num: PRICE_DEN,
                    price_den,
                },
            );
            i = i + 1;
        };
        levels
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

    /// Levels still resting in front of the cursor.
    public fun live_levels(pool_id: u64): u64 acquires Pools {
        if (!exists<Pools>(@bench)) {
            return 0
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return 0
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        vector::length(&pool.levels) - pool.cursor
    }

    /// Resting size, price numerator and price denominator of level `i`. An
    /// index past the end reads as an empty level priced at one.
    public fun level_at(pool_id: u64, i: u64): (u64, u64, u64) acquires Pools {
        if (!exists<Pools>(@bench)) {
            return (0, 1, 1)
        };
        let pools = borrow_global<Pools>(@bench);
        if (pools.n_pools == 0) {
            return (0, 1, 1)
        };
        let pool = table::borrow(&pools.pools, pool_id % pools.n_pools);
        if (i >= vector::length(&pool.levels)) {
            return (0, 1, 1)
        };
        let level = vector::borrow(&pool.levels, i);
        (level.size, level.price_num, level.price_den)
    }

    /// `(holds, forward)` for a hop from `asset_in` to `asset_out`: whether
    /// the book trades that pair at all, and which way round it holds it. A
    /// book that holds something else is not a venue for this hop, and saying
    /// so is what keeps a route to the legs it named.
    fun direction(pool: &Pool, asset_in: address, asset_out: address): (bool, bool) {
        if (pool.x == asset_in && pool.y == asset_out) { (true, true) }
        else if (pool.y == asset_in && pool.x == asset_out) { (true, false) }
        else { (false, false) }
    }

    /// Trade `amount_in` of `asset_in` through book `pool_id`, taking levels
    /// until it fills. A book that does not hold the pair trades nothing.
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
        let (holds, forward) = direction(pool, asset_in, asset_out);
        if (!holds) {
            return 0
        };
        let (reserve_in, reserve_out) =
            if (forward) { (pool.reserve_x, pool.reserve_y) }
            else { (pool.reserve_y, pool.reserve_x) };
        let budget = dexr_math::clamp_in(amount_in, reserve_in);
        if (budget == 0 || reserve_out == 0) {
            return 0
        };

        let n_levels = vector::length(&pool.levels);
        let cursor = pool.cursor;
        let remaining = budget;
        let amount_out = 0;
        let filled = 0;
        // The walk stops at the end of the level vector, so a trade deeper
        // than the book fills partially.
        while (remaining > 0 && cursor < n_levels) {
            let level = vector::borrow_mut(&mut pool.levels, cursor);
            let size = level.size;
            let affordable =
                dexr_math::mul_div(remaining, level.price_num, level.price_den);
            let take = dexr_math::min(affordable, size);
            if (take == 0) {
                cursor = cursor + 1;
            } else {
                let cost =
                    dexr_math::mul_div(take, level.price_den, level.price_num);
                if (cost > remaining) {
                    cost = remaining;
                };
                // A level that rounds to a free fill would never make
                // progress, so the walk always pays at least one unit.
                if (cost == 0) {
                    cost = 1;
                };
                level.size = size - take;
                amount_out = amount_out + take;
                remaining = remaining - cost;
                if (take == size) {
                    cursor = cursor + 1;
                    filled = filled + 1;
                };
            };
        };
        // Refilling on wrap puts the book back in front of the next hop, so
        // the venue never settles into quoting zero.
        if (cursor >= n_levels) {
            pool.levels = build_levels(n_levels, pool.level_size, pool.level_seed);
            pool.cursor = 0;
        } else {
            pool.cursor = cursor;
        };
        pool.filled_levels = pool.filled_levels + filled;

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
        let (holds, forward) = direction(pool, asset_in, asset_out);
        if (!holds) {
            return 0
        };
        let (reserve_in, reserve_out) =
            if (forward) { (pool.reserve_x, pool.reserve_y) }
            else { (pool.reserve_y, pool.reserve_x) };
        let remaining = dexr_math::clamp_in(amount_in, reserve_in);
        let n_levels = vector::length(&pool.levels);
        let cursor = pool.cursor;
        let amount_out = 0;
        while (remaining > 0 && cursor < n_levels) {
            let level = vector::borrow(&pool.levels, cursor);
            let size = level.size;
            let affordable =
                dexr_math::mul_div(remaining, level.price_num, level.price_den);
            let take = dexr_math::min(affordable, size);
            if (take == 0) {
                cursor = cursor + 1;
            } else {
                let cost =
                    dexr_math::mul_div(take, level.price_den, level.price_num);
                if (cost > remaining) {
                    cost = remaining;
                };
                if (cost == 0) {
                    cost = 1;
                };
                amount_out = amount_out + take;
                remaining = remaining - cost;
                if (take == size) {
                    cursor = cursor + 1;
                };
            };
        };
        if (reserve_out == 0) { 0 }
        else if (amount_out >= reserve_out) { reserve_out - 1 }
        else { amount_out }
    }

    /// Mint `amount` into both vaults and put the book back, so no venue
    /// drifts to a state where every trade clamps to a trivial amount.
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
        pool.levels = build_levels(
            vector::length(&pool.levels), pool.level_size, pool.level_seed
        );
        pool.cursor = 0;
    }
}
