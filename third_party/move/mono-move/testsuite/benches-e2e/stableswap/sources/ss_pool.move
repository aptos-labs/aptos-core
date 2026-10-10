/// StableSwap pools over real fungible assets, following the shape of Curve's
/// two- and three-coin pools.
///
/// A pool owns one `FungibleStore` per coin under a named object and keeps its
/// own balance vector alongside, so every swap reads the whole reserve set,
/// solves the invariant against it, and writes two of them back. Coins may
/// have different decimals; `ss_math` lifts them to a common precision before
/// the solve.
///
/// Amounts crossing an entry point are whole tokens. The pool converts them
/// with the coin's own scale, so a mixed-decimal pool sits at the composition
/// it was given rather than at one its decimals imply.
///
/// Every coin that enters a pool is withdrawn from the payer's own store, and
/// a mint runs only when that store is short. A benchmark account cannot be
/// left stuck, but it also cannot trade for free: the store debit is part of
/// the write set, and a trade the payer cannot cover shrinks to what it can.
module bench::ss_pool {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::{Self, FungibleStore, Metadata};
    use aptos_framework::object::{Self, Object};
    use bench::ss_amp;
    use bench::ss_assets;
    use bench::ss_math;

    friend bench::ss_lp;

    /// Only the package address can configure the protocol.
    const E_NOT_BENCH: u64 = 1;
    /// Pool ids are handed out in order from one.
    const E_BAD_POOL_ID: u64 = 2;
    /// A pool needs at least two and at most `MAX_COINS` coins.
    const E_BAD_COIN_COUNT: u64 = 3;
    /// A pool coin must have been created by `ss_assets` first.
    const E_UNKNOWN_ASSET: u64 = 4;
    /// Coin decimals outside the range the invariant math is bounded for.
    const E_BAD_DECIMALS: u64 = 5;

    const BPS: u64 = 10000;

    /// Widest pool the package builds. Curve's own pools stop at four coins,
    /// and past that a degenerate balance set stops fitting a u256.
    const MAX_COINS: u64 = 4;

    /// Decimal range a pool coin may have. Outside it the scaled balances
    /// leave the range `ss_math` is bounded for.
    const MIN_DECIMALS: u8 = 6;
    const MAX_DECIMALS: u8 = 10;

    /// Largest share of a reserve one swap may take, in basis points. Curve
    /// has no such cap; this one is what makes an oversized trade clamp
    /// instead of draining the pool.
    const MAX_SWAP_BPS: u64 = 500;

    /// Widest ratio between two scaled reserves a swap may leave behind. Past
    /// it the Newton loops stop converging inside their round bound, and the
    /// per-swap input cap is too small to refill the coin that got there.
    const MAX_SPREAD: u256 = 100;

    /// Largest fee a pool may carry, in basis points.
    const MAX_FEE_BPS: u64 = 100;

    /// How far past a shortfall a top-up mints. A trader that runs out pays
    /// from its own balance for the next several calls instead of minting on
    /// every one, so the common path is a plain store debit.
    const REFILL_MULTIPLE: u64 = 8;

    /// Floor on a pool's deposit composition, in basis points. Under it the
    /// invariant solve stops converging inside its round bound at the
    /// amplifications this package uses, and a swap would burn all 255 rounds.
    const MIN_SKEW_BP: u64 = 100;

    /// Ceiling on any raw balance the package moves, leaving room for the fee
    /// arithmetic to stay inside a u64.
    const MAX_RAW: u64 = 100000000000000000;

    struct LpKey has copy, drop, store {
        pool_id: u64,
        owner: address,
    }

    struct Pool has store {
        coins: vector<Object<Metadata>>,
        stores: vector<Object<FungibleStore>>,
        /// Multiplier lifting each coin to the common precision.
        rates: vector<u256>,
        /// Smallest unit of each coin, so whole-token amounts convert.
        units: vector<u64>,
        balances: vector<u64>,
        fee_bps: u64,
        /// Share of the lead coin's amount every other coin gets when the
        /// package deposits into this pool, in basis points. At `BPS` the
        /// pool holds equal value in every coin; below it the pool sits
        /// lopsided, which costs the invariant solve more Newton rounds.
        deposit_skew_bp: u64,
        lp_supply: u256,
    }

    struct Registry has key {
        pools: Table<u64, Pool>,
        n_pools: u64,
    }

    struct LpBook has key {
        balances: Table<LpKey, u256>,
    }

    // Configuration.

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Registry>(@bench)) {
            move_to(admin, Registry { pools: table::new(), n_pools: 0 });
        };
        if (!exists<LpBook>(@bench)) {
            move_to(admin, LpBook { balances: table::new() });
        };
        ss_amp::initialize(admin);
    }

    /// Create the next pool over the assets `ss_assets` holds for `symbols`.
    /// `a` is the raw amplification coefficient, `fee_bps` the swap fee, and
    /// `deposit_skew_bp` the composition the package's own deposits target.
    public entry fun create_pool(
        admin: &signer,
        pool_id: u64,
        symbols: vector<vector<u8>>,
        a: u64,
        fee_bps: u64,
        deposit_skew_bp: u64,
    ) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let n = vector::length(&symbols);
        assert!(n >= 2 && n <= MAX_COINS, E_BAD_COIN_COUNT);

        let pool_ctor = object::create_named_object(admin, pool_seed(pool_id));
        let pool_signer = object::generate_signer(&pool_ctor);

        let coins = vector::empty<Object<Metadata>>();
        let stores = vector::empty<Object<FungibleStore>>();
        let rates = vector::empty<u256>();
        let units = vector::empty<u64>();
        let balances = vector::empty<u64>();
        let k = 0;
        while (k < n) {
            let symbol = *vector::borrow(&symbols, k);
            assert!(ss_assets::asset_exists(@bench, symbol), E_UNKNOWN_ASSET);
            let asset = ss_assets::asset(@bench, symbol);
            let decimals = ss_assets::decimals(asset);
            assert!(
                decimals >= MIN_DECIMALS && decimals <= MAX_DECIMALS,
                E_BAD_DECIMALS,
            );
            let store_ctor = object::create_named_object(&pool_signer, symbol);
            vector::push_back(&mut coins, asset);
            vector::push_back(
                &mut stores, fungible_asset::create_store(&store_ctor, asset));
            vector::push_back(&mut rates, ss_math::rate_for_decimals(decimals));
            vector::push_back(&mut units, ss_math::unit_for_decimals(decimals));
            vector::push_back(&mut balances, 0);
            k = k + 1;
        };

        let registry = borrow_global_mut<Registry>(@bench);
        assert!(pool_id == registry.n_pools + 1, E_BAD_POOL_ID);
        table::add(&mut registry.pools, pool_id, Pool {
            coins,
            stores,
            rates,
            units,
            balances,
            fee_bps: if (fee_bps > MAX_FEE_BPS) { MAX_FEE_BPS } else { fee_bps },
            deposit_skew_bp: clamp_skew(deposit_skew_bp),
            lp_supply: 0,
        });
        registry.n_pools = pool_id;
        ss_amp::register(pool_id, a);
    }

    /// Fund a pool from the publisher at the pool's own composition. One pool
    /// per transaction keeps a seed inside the per-transaction execution
    /// limit.
    public entry fun seed_liquidity(
        admin: &signer, pool_id: u64, units: u64
    ) acquires Registry, LpBook {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        add_at_pool_skew(signer::address_of(admin), pool_id, units);
    }

    // Benchmark entry points.

    /// Bring a fresh account to the state the mix assumes: funded in every
    /// coin of every pool, and holding LP in each.
    public entry fun bench_onboard(
        user: &signer, fund_units: u64, deposit_units: u64
    ) acquires Registry, LpBook {
        if (!exists<Registry>(@bench)) {
            return
        };
        let owner = signer::address_of(user);
        let n_pools = borrow_global<Registry>(@bench).n_pools;
        let pool_id = 1;
        while (pool_id <= n_pools) {
            faucet_pool_coins(owner, pool_id, fund_units);
            add_at_pool_skew(owner, pool_id, deposit_units);
            pool_id = pool_id + 1;
        }
    }

    /// Swap whole tokens of coin `i` for coin `j`, paid out of the caller's
    /// own balance. The input is clamped to a fraction of the source reserve
    /// and then to what the caller can pay, so no caller state and no coin
    /// index can make this fail.
    public entry fun bench_swap(
        user: &signer, pool_id: u64, i: u64, j: u64, dx_units: u64
    ) acquires Registry {
        internal_swap(signer::address_of(user), pool_id, i, j, dx_units);
    }

    /// Returns the amount of coin `j` paid out, in that coin's smallest unit.
    public fun internal_swap(
        owner: address, pool_id: u64, i: u64, j: u64, dx_units: u64
    ): u64 acquires Registry {
        if (!exists<Registry>(@bench)) {
            return 0
        };
        let amp = ss_amp::amp(pool_id);
        let registry = borrow_global_mut<Registry>(@bench);
        if (!table::contains(&registry.pools, pool_id)) {
            return 0
        };
        let pool = table::borrow_mut(&mut registry.pools, pool_id);
        let n = vector::length(&pool.coins);
        if (n < 2) {
            return 0
        };
        let i = i % n;
        let j = j % n;
        if (i == j) {
            j = (i + 1) % n;
        };

        let reserve_i = *vector::borrow(&pool.balances, i);
        let dx = to_raw(dx_units, *vector::borrow(&pool.units, i));
        let cap = reserve_i / (BPS / MAX_SWAP_BPS);
        if (dx > cap) {
            dx = cap;
        };
        let headroom = headroom_of(reserve_i);
        if (dx > headroom) {
            dx = headroom;
        };
        if (dx == 0) {
            return 0
        };

        // The trade is sized against the reserve first and paid for second, so
        // a trader who cannot cover it trades smaller rather than aborting.
        dx = pay_in(
            *vector::borrow(&pool.coins, i),
            *vector::borrow(&pool.stores, i),
            owner,
            dx,
        );
        if (dx == 0) {
            return 0
        };

        let xp = ss_math::xp_mem(&pool.balances, &pool.rates);
        let x = *vector::borrow(&xp, i) + (dx as u256)
            * *vector::borrow(&pool.rates, i);
        let y = ss_math::get_y(i, j, x, &xp, amp);
        let dy = payout(pool, j, &xp, y);

        *vector::borrow_mut(&mut pool.balances, i) = reserve_i + dx;
        pay_out(pool, owner, j, dy);
        dy
    }

    // Liquidity, driven by `ss_lp`.

    /// Move `amounts` of each coin in from `owner`, in the coins' smallest
    /// units, and mint the LP shares the invariant grew by. `get_D` runs once
    /// before and once after, which is what makes a deposit the most expensive
    /// flow here.
    public(friend) fun add_liquidity(
        owner: address, pool_id: u64, amounts: vector<u64>
    ): u256 acquires Registry, LpBook {
        if (!exists<Registry>(@bench)) {
            return 0
        };
        let amp = ss_amp::amp(pool_id);
        let registry = borrow_global_mut<Registry>(@bench);
        if (!table::contains(&registry.pools, pool_id)) {
            return 0
        };
        let pool = table::borrow_mut(&mut registry.pools, pool_id);
        let n = vector::length(&pool.coins);
        let given = vector::length(&amounts);
        let d0 = ss_math::get_D(
            &ss_math::xp_mem(&pool.balances, &pool.rates), amp);

        let moved = false;
        let k = 0;
        while (k < n) {
            let amount = if (k < given) { *vector::borrow(&amounts, k) } else { 0 };
            let reserve = *vector::borrow(&pool.balances, k);
            let headroom = headroom_of(reserve);
            if (amount > headroom) {
                amount = headroom;
            };
            if (amount > 0) {
                let paid = pay_in(
                    *vector::borrow(&pool.coins, k),
                    *vector::borrow(&pool.stores, k),
                    owner,
                    amount,
                );
                if (paid > 0) {
                    *vector::borrow_mut(&mut pool.balances, k) = reserve + paid;
                    moved = true;
                };
            };
            k = k + 1;
        };
        if (!moved) {
            return 0
        };

        let d1 = ss_math::get_D(
            &ss_math::xp_mem(&pool.balances, &pool.rates), amp);
        if (d1 <= d0) {
            return 0
        };
        let minted = if (pool.lp_supply == 0 || d0 == 0) {
            d1 - d0
        } else {
            pool.lp_supply * (d1 - d0) / d0
        };
        if (minted == 0) {
            return 0
        };
        pool.lp_supply = pool.lp_supply + minted;
        credit_lp(owner, pool_id, minted);
        minted
    }

    /// Burn `lp_bps` basis points of the caller's LP into coin `j` alone. The
    /// withdrawal solves the invariant twice, once for the current `D` and
    /// once for coin `j` at the reduced one.
    public(friend) fun remove_liquidity_one_coin(
        owner: address, pool_id: u64, j: u64, lp_bps: u64
    ): u64 acquires Registry, LpBook {
        if (!exists<Registry>(@bench) || !exists<LpBook>(@bench)) {
            return 0
        };
        let held = lp_balance(owner, pool_id);
        if (held == 0) {
            return 0
        };
        let bps = if (lp_bps > BPS) { BPS } else { lp_bps };
        let burn = held * (bps as u256) / (BPS as u256);
        if (burn == 0) {
            return 0
        };

        let amp = ss_amp::amp(pool_id);
        let registry = borrow_global_mut<Registry>(@bench);
        if (!table::contains(&registry.pools, pool_id)) {
            return 0
        };
        let pool = table::borrow_mut(&mut registry.pools, pool_id);
        let n = vector::length(&pool.coins);
        if (n == 0 || pool.lp_supply == 0) {
            return 0
        };
        if (burn > pool.lp_supply) {
            burn = pool.lp_supply;
        };
        let j = j % n;

        let xp = ss_math::xp_mem(&pool.balances, &pool.rates);
        let d0 = ss_math::get_D(&xp, amp);
        let d1 = d0 - d0 * burn / pool.lp_supply;
        let y = ss_math::get_y_D(j, &xp, amp, d1);
        let dy = payout(pool, j, &xp, y);

        pool.lp_supply = pool.lp_supply - burn;
        pay_out(pool, owner, j, dy);
        debit_lp(owner, pool_id, burn);
        dy
    }

    // Views.

    public fun num_pools(): u64 acquires Registry {
        if (!exists<Registry>(@bench)) { 0 }
        else { borrow_global<Registry>(@bench).n_pools }
    }

    public fun pool_exists(pool_id: u64): bool acquires Registry {
        exists<Registry>(@bench)
            && table::contains(&borrow_global<Registry>(@bench).pools, pool_id)
    }

    public fun num_coins(pool_id: u64): u64 acquires Registry {
        if (!pool_exists(pool_id)) {
            return 0
        };
        let pool = table::borrow(&borrow_global<Registry>(@bench).pools, pool_id);
        vector::length(&pool.coins)
    }

    public fun reserve(pool_id: u64, k: u64): u64 acquires Registry {
        let n = num_coins(pool_id);
        if (n == 0) {
            return 0
        };
        let pool = table::borrow(&borrow_global<Registry>(@bench).pools, pool_id);
        *vector::borrow(&pool.balances, k % n)
    }

    public fun coin_at(pool_id: u64, k: u64): Object<Metadata> acquires Registry {
        let pool = table::borrow(&borrow_global<Registry>(@bench).pools, pool_id);
        *vector::borrow(&pool.coins, k % vector::length(&pool.coins))
    }

    public fun deposit_skew_bp(pool_id: u64): u64 acquires Registry {
        if (!pool_exists(pool_id)) {
            return BPS
        };
        let pool = table::borrow(&borrow_global<Registry>(@bench).pools, pool_id);
        pool.deposit_skew_bp
    }

    public fun lp_supply(pool_id: u64): u256 acquires Registry {
        if (!pool_exists(pool_id)) {
            return 0
        };
        table::borrow(&borrow_global<Registry>(@bench).pools, pool_id).lp_supply
    }

    public fun lp_balance(owner: address, pool_id: u64): u256 acquires LpBook {
        if (!exists<LpBook>(@bench)) {
            return 0
        };
        let book = &borrow_global<LpBook>(@bench).balances;
        let key = LpKey { pool_id, owner };
        if (table::contains(book, key)) { *table::borrow(book, key) } else { 0 }
    }

    /// Whole-token `units` of coin `k`, in that coin's smallest unit.
    public fun raw_amount(pool_id: u64, k: u64, units: u64): u64 acquires Registry {
        let n = num_coins(pool_id);
        if (n == 0) {
            return 0
        };
        let pool = table::borrow(&borrow_global<Registry>(@bench).pools, pool_id);
        to_raw(units, *vector::borrow(&pool.units, k % n))
    }

    // Internals.

    /// Amount of coin `j` a solve from `xp[j]` down to `y` pays out, net of
    /// the pool fee and clamped so coin `j` keeps at least `MAX_SPREAD` of the
    /// pool's largest scaled reserve, and its store never empties.
    fun payout(pool: &Pool, j: u64, xp: &vector<u256>, y: u256): u64 {
        let xp_j = *vector::borrow(xp, j);
        let gross = if (xp_j > y + 1) { xp_j - y - 1 } else { 0 };
        let rate = *vector::borrow(&pool.rates, j);
        let scaled = gross / rate;
        let reserve = *vector::borrow(&pool.balances, j);
        let dy = if (scaled > (reserve as u256)) { reserve } else { (scaled as u64) };
        dy = dy - (((dy as u256) * (pool.fee_bps as u256) / (BPS as u256)) as u64);

        let floor = widest(xp) / MAX_SPREAD / rate;
        if ((reserve as u256) <= floor) {
            return 0
        };
        let room = reserve - (floor as u64);
        if (dy > room) {
            dy = room;
        };
        if (dy >= reserve) {
            dy = if (reserve > 1) { reserve - 1 } else { 0 };
        };
        let held = ss_assets::balance(*vector::borrow(&pool.stores, j));
        if (dy > held) { held } else { dy }
    }

    fun widest(xp: &vector<u256>): u256 {
        let n = vector::length(xp);
        let widest = 0u256;
        let k = 0;
        while (k < n) {
            let x = *vector::borrow(xp, k);
            if (x > widest) {
                widest = x;
            };
            k = k + 1;
        };
        widest
    }

    fun pay_out(pool: &mut Pool, owner: address, j: u64, dy: u64) {
        if (dy == 0) {
            return
        };
        let coin = *vector::borrow(&pool.coins, j);
        let store = *vector::borrow(&pool.stores, j);
        ss_assets::deposit(
            coin,
            ss_assets::primary_store(owner, coin),
            ss_assets::withdraw(coin, store, dy),
        );
        let reserve = vector::borrow_mut(&mut pool.balances, j);
        *reserve = *reserve - dy;
    }

    /// Deposit `units` whole tokens of coin 0 and the pool's skew share of
    /// every other coin. Every deposit the package makes goes through here, so
    /// a lopsided pool stays lopsided instead of being averaged back to
    /// balance by its own traffic.
    fun add_at_pool_skew(
        owner: address, pool_id: u64, units: u64
    ) acquires Registry, LpBook {
        let n = num_coins(pool_id);
        if (n == 0) {
            return
        };
        let skew = (deposit_skew_bp(pool_id) as u256);
        let amounts = vector::empty<u64>();
        let k = 0;
        while (k < n) {
            let share = if (k == 0) {
                units
            } else {
                (((units as u256) * skew / (BPS as u256)) as u64)
            };
            vector::push_back(&mut amounts, raw_amount(pool_id, k, share));
            k = k + 1;
        };
        add_liquidity(owner, pool_id, amounts);
    }

    /// Move `amount` of `coin` from `owner`'s primary store into `store`,
    /// minting only what `owner` is short and overshooting the shortfall so
    /// the next several calls are plain debits. Returns what moved, which is
    /// below `amount` only when the mint could not close the gap.
    fun pay_in(
        coin: Object<Metadata>,
        store: Object<FungibleStore>,
        owner: address,
        amount: u64,
    ): u64 {
        if (amount == 0) {
            return 0
        };
        let from = ss_assets::primary_store(owner, coin);
        let held = ss_assets::balance(from);
        if (held < amount) {
            let refill = (amount - held) * REFILL_MULTIPLE;
            let room = headroom_of(held);
            if (refill > room) {
                refill = room;
            };
            ss_assets::faucet(coin, owner, refill);
            held = held + refill;
        };
        let moved = if (amount > held) { held } else { amount };
        if (moved == 0) {
            return 0
        };
        ss_assets::deposit(coin, store, ss_assets::withdraw(coin, from, moved));
        moved
    }

    fun clamp_skew(bp: u64): u64 {
        if (bp < MIN_SKEW_BP) { MIN_SKEW_BP }
        else if (bp > BPS) { BPS }
        else { bp }
    }

    fun faucet_pool_coins(
        owner: address, pool_id: u64, units: u64
    ) acquires Registry {
        let n = num_coins(pool_id);
        let k = 0;
        while (k < n) {
            let amount = raw_amount(pool_id, k, units);
            ss_assets::faucet(coin_at(pool_id, k), owner, amount);
            k = k + 1;
        }
    }

    fun credit_lp(owner: address, pool_id: u64, amount: u256) acquires LpBook {
        let book = &mut borrow_global_mut<LpBook>(@bench).balances;
        let key = LpKey { pool_id, owner };
        if (table::contains(book, key)) {
            let slot = table::borrow_mut(book, key);
            *slot = *slot + amount;
        } else {
            table::add(book, key, amount);
        }
    }

    fun debit_lp(owner: address, pool_id: u64, amount: u256) acquires LpBook {
        let book = &mut borrow_global_mut<LpBook>(@bench).balances;
        let key = LpKey { pool_id, owner };
        if (!table::contains(book, key)) {
            return
        };
        let slot = table::borrow_mut(book, key);
        *slot = if (*slot > amount) { *slot - amount } else { 0 };
    }

    /// How much more a reserve may take before the fee arithmetic runs out of
    /// u64 room.
    fun headroom_of(reserve: u64): u64 {
        if (reserve >= MAX_RAW) { 0 } else { MAX_RAW - reserve }
    }

    fun to_raw(units: u64, scale: u64): u64 {
        let raw = (units as u256) * (scale as u256);
        if (raw > (MAX_RAW as u256)) { MAX_RAW } else { (raw as u64) }
    }

    fun pool_seed(pool_id: u64): vector<u8> {
        let seed = b"ss_pool";
        vector::append(&mut seed, bcs::to_bytes(&pool_id));
        seed
    }
}
