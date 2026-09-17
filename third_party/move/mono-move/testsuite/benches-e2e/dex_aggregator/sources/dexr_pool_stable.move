/// Amplified two-coin pools, shaped after the Curve-style venues a Panora
/// route uses for correlated pairs.
///
/// Storage matches `dexr_pool_cpmm`. What differs is the price: two bounded
/// Newton loops per hop instead of one division, which is why a stable leg
/// costs more interpreter time than a constant-product leg of the same size.
module bench::dexr_pool_stable {
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
    const PREFIX: vector<u8> = b"dexr_stable";

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    struct Pool has store {
        x: address,
        y: address,
        store_x: Object<FungibleStore>,
        store_y: Object<FungibleStore>,
        reserve_x: u64,
        reserve_y: u64,
        fee_bps: u64,
        amp: u64,
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
        amp: u64,
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
                amp,
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
        amp: u64,
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
            let y = *vector::borrow(&assets, (state + 2) % n_assets);
            // Amplification differs per pool, so the Newton trip count cannot
            // be folded to one constant across the backend.
            create_pool(
                admin,
                x,
                y,
                reserve + state % (reserve / 4 + 1),
                fee_bps,
                amp + state % 64,
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
        let amount_in = dexr_math::clamp_in(amount_in, reserve_in);
        if (amount_in == 0 || reserve_out == 0) {
            return 0
        };
        let amount_out = dexr_math::stable_out(
            amount_in, reserve_in, reserve_out, pool.amp, pool.fee_bps
        );
        if (forward) {
            pool.reserve_x = reserve_in + amount_in;
            pool.reserve_y = reserve_out - amount_out;
        } else {
            pool.reserve_y = reserve_in + amount_in;
            pool.reserve_x = reserve_out - amount_out;
        };
        let (in_asset, in_store, out_asset, out_store) =
            if (forward) { (pool.x, pool.store_x, pool.y, pool.store_y) }
            else { (pool.y, pool.store_y, pool.x, pool.store_x) };
        dexr_assets::settle(
            user, in_asset, in_store, amount_in, out_asset, out_store, amount_out
        );
        amount_out
    }

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
        dexr_math::stable_out(
            dexr_math::clamp_in(amount_in, reserve_in),
            reserve_in,
            reserve_out,
            pool.amp,
            pool.fee_bps,
        )
    }

    /// Mint `amount` into both vaults so no pool drifts to a reserve where
    /// every trade clamps to a trivial amount.
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
    }
}
