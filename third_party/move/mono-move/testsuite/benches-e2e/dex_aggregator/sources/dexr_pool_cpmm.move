/// Constant product pools, shaped after the PancakeSwap v2 and Thala pairs a
/// Panora route lands on most often.
///
/// Every pool keeps its two sides in real fungible asset vaults and mirrors
/// their balances in `reserve_x` / `reserve_y`, so a quote never has to touch
/// the stores and a swap never has to reconcile them.
module bench::dexr_pool_cpmm {
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
    const PREFIX: vector<u8> = b"dexr_cpmm";

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
        admin: &signer, x: address, y: address, reserve: u64, fee_bps: u64
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
            // Pool `id` holds assets `id` and `id + 1`. A route picks the pool
            // each of its hops lands on before it knows what that pool holds,
            // so which pair a pool holds has to follow from its id.
            let id = n_pools();
            let x = *vector::borrow(&assets, id % n_assets);
            let y = *vector::borrow(&assets, (id + 1) % n_assets);
            // Sizes differ per pool so a route's hops do not all price the
            // same way, which keeps the mix off a single hot path.
            create_pool(admin, x, y, reserve + state % (reserve / 4 + 1), fee_bps);
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

    /// `(holds, forward)` for a hop from `asset_in` to `asset_out`: whether
    /// the pool trades that pair at all, and which way round it holds it. A
    /// pool that holds something else is not a venue for this hop, and saying
    /// so is what keeps a route to the legs it named.
    fun direction(pool: &Pool, asset_in: address, asset_out: address): (bool, bool) {
        if (pool.x == asset_in && pool.y == asset_out) { (true, true) }
        else if (pool.y == asset_in && pool.x == asset_out) { (true, false) }
        else { (false, false) }
    }

    /// Trade `amount_in` of `asset_in` through pool `pool_id`. An id past the
    /// end of the table wraps, so an out-of-range id still trades. A pool that
    /// does not hold the pair trades nothing.
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
        let amount_in = dexr_math::clamp_in(amount_in, reserve_in);
        if (amount_in == 0 || reserve_out == 0) {
            return 0
        };
        let amount_out =
            dexr_math::cpmm_out(amount_in, reserve_in, reserve_out, pool.fee_bps);
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
        let (holds, forward) = direction(pool, asset_in, asset_out);
        if (!holds) {
            return 0
        };
        let (reserve_in, reserve_out) =
            if (forward) { (pool.reserve_x, pool.reserve_y) }
            else { (pool.reserve_y, pool.reserve_x) };
        dexr_math::cpmm_out(
            dexr_math::clamp_in(amount_in, reserve_in),
            reserve_in,
            reserve_out,
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
