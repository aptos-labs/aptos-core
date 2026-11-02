/// The pool itself: state, entry points and the swap loop.
///
/// A swap walks initialized ticks one at a time, so its cost scales with how
/// far the price moves rather than with the number of positions. That is the
/// shape this benchmark is meant to exercise.
module bench::clmm_pool {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_framework::fungible_asset::{Self, FungibleStore, Metadata};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use bench::clmm_liquidity_math;
    use bench::clmm_position::{Self, PositionKey, Positions};
    use bench::clmm_swap_math;
    use bench::clmm_tick::{Self, Ticks};
    use bench::clmm_tick_bitmap::{Self, BitMap};
    use bench::clmm_assets;
    use bench::clmm_tick_math;

    /// A pool already exists for this token pair and configuration.
    const EPOOL_EXISTS: u64 = 1;
    /// No pool at that address.
    const ENO_POOL: u64 = 2;
    /// A pool needs two different tokens.
    const ESAME_TOKEN: u64 = 3;
    /// Price limit is on the wrong side of the current price.
    const EINVALID_PRICE_LIMIT: u64 = 4;
    /// Minting or burning zero liquidity.
    const EZERO_LIQUIDITY: u64 = 5;
    /// A token amount left `u64`.
    const EAMOUNT_OVERFLOW: u64 = 6;
    /// Tick spacing of zero.
    const EZERO_TICK_SPACING: u64 = 7;
    /// Swap produced nothing.
    const EEMPTY_SWAP: u64 = 8;
    /// Seeding a pool that is not owned by the caller.
    const ENOT_POOL_ADMIN: u64 = 9;
    /// Fee rate at or above 100%.
    const EINVALID_FEE_RATE: u64 = 10;

    const MAX_U64: u256 = 18446744073709551615;
    const MAX_U128: u256 = 340282366920938463463374607431768211455;

    /// House linear congruential generator, shared across these benchmarks so
    /// their access patterns are comparable.
    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    /// Tick spacings a seeded pool spreads `tick_density` boundaries over. A
    /// density of one puts them this far apart and the maximum puts them one
    /// spacing apart, so the same trade meets twenty times as many.
    const DENSITY_SPACINGS: u32 = 20;

    /// Fraction of the reserve a single hop of a route may sell into. Taking
    /// the whole reserve would park the price at the bound and leave the rest
    /// of the route with nothing to trade.
    const HOP_RESERVE_FRACTION: u64 = 20;

    /// The largest swap the harness sends. Against the backstop it moves the
    /// price about 1200 ticks, which is one seeded boundary at the lowest
    /// density and twenty at the highest, so lowering it is what stops the
    /// density knob doing anything.
    const BENCH_SWAP_SIZE: u64 = 60000000000000;

    /// The tick range the harness backstops a pool over.
    ///
    /// As wide as it can go, because the price random-walks under the mix and
    /// anything narrower is a stretch of zero liquidity it can reach. The
    /// ceiling is `mul_div`: a position's token A costs
    /// `(liquidity << 96) * (sqrt_hi - sqrt_lo) / sqrt_hi`, and at the
    /// harness's liquidity that product leaves `u256` between 180000 and
    /// 197000 ticks. This keeps a few bits in hand.
    const BENCH_BACKSTOP_TICK: i32 = 150000;

    /// The density the harness seeds its pools at. The whole workload turns on
    /// a swap crossing dozens of ticks rather than a few, so this is the knob
    /// that decides whether it measures what it is named for.
    const BENCH_TICK_DENSITY: u32 = 20;

    struct Pool has key {
        admin: address,
        token_a: Object<Metadata>,
        token_b: Object<Metadata>,
        vault_a: Object<FungibleStore>,
        vault_b: Object<FungibleStore>,
        sqrt_price: u128,
        tick: i32,
        liquidity: u128,
        fee_rate: u64,
        tick_spacing: u32,
        max_liquidity_per_tick: u128,
        fee_growth_global_a: u128,
        fee_growth_global_b: u128,
        tick_density: u32,
        last_swap_crossings: u64,
        band_lower: i32,
        band_upper: i32,
        ticks: Ticks,
        tick_bitmap: BitMap,
        positions: Positions,
        extend_ref: ExtendRef,
    }

    /// The admin's pools in creation order. A route names its hops by index,
    /// so a caller does not have to derive every pool address it touches.
    struct PoolRegistry has key {
        pools: vector<address>,
    }

    /// Pools are named objects, so their address falls out of the pair and the
    /// configuration and a caller never has to be told it.
    public fun pool_seed(
        token_a: Object<Metadata>, token_b: Object<Metadata>, fee_rate: u64, tick_spacing: u32
    ): vector<u8> {
        let seed = b"clmm_swap_pool";
        vector::append(&mut seed, bcs::to_bytes(&object::object_address(&token_a)));
        vector::append(&mut seed, bcs::to_bytes(&object::object_address(&token_b)));
        vector::append(&mut seed, bcs::to_bytes(&fee_rate));
        vector::append(&mut seed, bcs::to_bytes(&tick_spacing));
        seed
    }

    #[view]
    public fun pool_address(
        admin: address,
        token_a: Object<Metadata>,
        token_b: Object<Metadata>,
        fee_rate: u64,
        tick_spacing: u32
    ): address {
        object::create_object_address(&admin, pool_seed(token_a, token_b, fee_rate, tick_spacing))
    }

    public entry fun create_pool(
        admin: &signer,
        token_a: Object<Metadata>,
        token_b: Object<Metadata>,
        fee_rate: u64,
        tick_spacing: u32,
        initial_sqrt_price: u128
    ) acquires PoolRegistry {
        assert!(token_a != token_b, ESAME_TOKEN);
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        assert!(fee_rate < clmm_swap_math::fee_rate_denominator(), EINVALID_FEE_RATE);
        let admin_address = signer::address_of(admin);
        let address = pool_address(admin_address, token_a, token_b, fee_rate, tick_spacing);
        assert!(!exists<Pool>(address), EPOOL_EXISTS);

        let constructor_ref =
            &object::create_named_object(admin, pool_seed(token_a, token_b, fee_rate, tick_spacing));
        let pool_signer = &object::generate_signer(constructor_ref);
        let vault_a = fungible_asset::create_store(
            &object::create_named_object(pool_signer, b"vault_a"), token_a
        );
        let vault_b = fungible_asset::create_store(
            &object::create_named_object(pool_signer, b"vault_b"), token_b
        );
        move_to(
            pool_signer,
            Pool {
                admin: admin_address,
                token_a,
                token_b,
                vault_a,
                vault_b,
                sqrt_price: initial_sqrt_price,
                tick: clmm_tick_math::get_tick_at_sqrt_price(initial_sqrt_price),
                liquidity: 0,
                fee_rate,
                tick_spacing,
                max_liquidity_per_tick: clmm_tick::max_liquidity_per_tick(tick_spacing),
                fee_growth_global_a: 0,
                fee_growth_global_b: 0,
                tick_density: 0,
                last_swap_crossings: 0,
                band_lower: 0,
                band_upper: 0,
                ticks: clmm_tick::new(),
                tick_bitmap: clmm_tick_bitmap::new(),
                positions: clmm_position::new(),
                extend_ref: object::generate_extend_ref(constructor_ref),
            }
        );

        if (!exists<PoolRegistry>(admin_address)) {
            move_to(admin, PoolRegistry { pools: vector::empty() });
        };
        vector::push_back(&mut borrow_global_mut<PoolRegistry>(admin_address).pools, address);
    }

    //
    // Liquidity.
    //

    /// Apply a liquidity change to the ticks, the bitmap and the position, and
    /// report the token amounts it moves.
    fun modify_position(
        pool: &mut Pool, key: PositionKey, tick_lower: i32, tick_upper: i32, liquidity_delta: i128
    ): (u64, u64) {
        clmm_tick_math::check_tick_range(tick_lower, tick_upper, pool.tick_spacing);

        let flipped_lower = clmm_tick::update(
            &mut pool.ticks,
            tick_lower,
            pool.tick,
            liquidity_delta,
            pool.fee_growth_global_a,
            pool.fee_growth_global_b,
            false,
            pool.max_liquidity_per_tick
        );
        let flipped_upper = clmm_tick::update(
            &mut pool.ticks,
            tick_upper,
            pool.tick,
            liquidity_delta,
            pool.fee_growth_global_a,
            pool.fee_growth_global_b,
            true,
            pool.max_liquidity_per_tick
        );
        if (flipped_lower) {
            clmm_tick_bitmap::flip_tick(&mut pool.tick_bitmap, tick_lower, pool.tick_spacing);
        };
        if (flipped_upper) {
            clmm_tick_bitmap::flip_tick(&mut pool.tick_bitmap, tick_upper, pool.tick_spacing);
        };

        let (inside_a, inside_b) = clmm_tick::get_fee_growth_inside(
            &pool.ticks,
            tick_lower,
            tick_upper,
            pool.tick,
            pool.fee_growth_global_a,
            pool.fee_growth_global_b
        );
        clmm_position::update(&mut pool.positions, key, liquidity_delta, inside_a, inside_b);

        if (liquidity_delta < 0i128) {
            if (flipped_lower) {
                clmm_tick::clear(&mut pool.ticks, tick_lower);
            };
            if (flipped_upper) {
                clmm_tick::clear(&mut pool.ticks, tick_upper);
            };
        };

        let magnitude =
            if (liquidity_delta < 0i128) {
                (-liquidity_delta) as u128
            } else {
                liquidity_delta as u128
            };
        let round_up = liquidity_delta > 0i128;
        let sqrt_lower = clmm_tick_math::get_sqrt_price_at_tick(tick_lower);
        let sqrt_upper = clmm_tick_math::get_sqrt_price_at_tick(tick_upper);

        // Which tokens a position holds depends on where the price sits
        // relative to its range: all token A above it, all token B below it,
        // and a mix in between.
        let (amount_a, amount_b) =
            if (pool.tick < tick_lower) {
                (
                    clmm_liquidity_math::get_amount_a_delta(
                        sqrt_lower, sqrt_upper, magnitude, round_up
                    ),
                    0u256
                )
            } else if (pool.tick < tick_upper) {
                if (liquidity_delta != 0i128) {
                    pool.liquidity =
                        clmm_liquidity_math::add_delta(pool.liquidity, liquidity_delta);
                };
                (
                    clmm_liquidity_math::get_amount_a_delta(
                        pool.sqrt_price, sqrt_upper, magnitude, round_up
                    ),
                    clmm_liquidity_math::get_amount_b_delta(
                        sqrt_lower, pool.sqrt_price, magnitude, round_up
                    )
                )
            } else {
                (
                    0u256,
                    clmm_liquidity_math::get_amount_b_delta(
                        sqrt_lower, sqrt_upper, magnitude, round_up
                    )
                )
            };
        assert!(amount_a <= MAX_U64 && amount_b <= MAX_U64, EAMOUNT_OVERFLOW);
        ((amount_a as u64), (amount_b as u64))
    }

    public entry fun mint(
        owner: &signer, pool_id: address, tick_lower: i32, tick_upper: i32, liquidity: u128
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        assert!(liquidity != 0, EZERO_LIQUIDITY);
        let owner_address = signer::address_of(owner);
        let pool = borrow_global_mut<Pool>(pool_id);
        let key = clmm_position::key(owner_address, tick_lower, tick_upper);
        let (amount_a, amount_b) =
            modify_position(pool, key, tick_lower, tick_upper, liquidity as i128);
        let (vault_a, vault_b) = (pool.vault_a, pool.vault_b);
        let (token_a, token_b) = (pool.token_a, pool.token_b);
        pay_in(owner, vault_a, token_a, amount_a);
        pay_in(owner, vault_b, token_b, amount_b);
    }

    /// Remove liquidity, crediting the principal to the position rather than
    /// paying it out. `collect` moves the tokens.
    public entry fun burn(
        owner: &signer, pool_id: address, tick_lower: i32, tick_upper: i32, liquidity: u128
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        assert!(liquidity != 0, EZERO_LIQUIDITY);
        let owner_address = signer::address_of(owner);
        let pool = borrow_global_mut<Pool>(pool_id);
        let key = clmm_position::key(owner_address, tick_lower, tick_upper);
        let (amount_a, amount_b) =
            modify_position(pool, key, tick_lower, tick_upper, -(liquidity as i128));
        clmm_position::credit(&mut pool.positions, key, amount_a, amount_b);
    }

    /// Settle a position's fees without changing its liquidity.
    public entry fun poke(
        owner: &signer, pool_id: address, tick_lower: i32, tick_upper: i32
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let owner_address = signer::address_of(owner);
        let pool = borrow_global_mut<Pool>(pool_id);
        let key = clmm_position::key(owner_address, tick_lower, tick_upper);
        modify_position(pool, key, tick_lower, tick_upper, 0i128);
    }

    public entry fun collect(
        owner: &signer,
        pool_id: address,
        tick_lower: i32,
        tick_upper: i32,
        requested_a: u64,
        requested_b: u64
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let owner_address = signer::address_of(owner);
        let pool = borrow_global_mut<Pool>(pool_id);
        let key = clmm_position::key(owner_address, tick_lower, tick_upper);
        let (taken_a, taken_b) =
            clmm_position::collect(&mut pool.positions, key, requested_a, requested_b);
        let (vault_a, vault_b) = (pool.vault_a, pool.vault_b);
        let pool_signer = object::generate_signer_for_extending(&pool.extend_ref);
        pay_out(&pool_signer, vault_a, owner_address, taken_a);
        pay_out(&pool_signer, vault_b, owner_address, taken_b);
    }

    //
    // Swaps.
    //

    /// Walk initialized ticks until the amount runs out or the price limit is
    /// reached, returning `(gross_in, amount_out)`.
    fun run_swap(
        pool: &mut Pool,
        a_to_b: bool,
        amount_specified: u64,
        exact_in: bool,
        sqrt_price_limit: u128
    ): (u64, u64) {
        if (a_to_b) {
            assert!(
                sqrt_price_limit < pool.sqrt_price
                    && sqrt_price_limit > clmm_tick_math::min_sqrt_price(),
                EINVALID_PRICE_LIMIT
            );
        } else {
            assert!(
                sqrt_price_limit > pool.sqrt_price
                    && sqrt_price_limit < clmm_tick_math::max_sqrt_price(),
                EINVALID_PRICE_LIMIT
            );
        };

        let remaining = amount_specified;
        let gross_in = 0u64;
        let total_out = 0u64;
        let crossings = 0u64;
        while (remaining != 0 && pool.sqrt_price != sqrt_price_limit) {
            let sqrt_start = pool.sqrt_price;
            let (next_tick, initialized) =
                clmm_tick_bitmap::next_initialized_tick_within_one_word(
                    &pool.tick_bitmap, pool.tick, pool.tick_spacing, a_to_b
                );
            if (next_tick < clmm_tick_math::min_tick()) {
                next_tick = clmm_tick_math::min_tick();
            };
            if (next_tick > clmm_tick_math::max_tick()) {
                next_tick = clmm_tick_math::max_tick();
            };
            let sqrt_next = clmm_tick_math::get_sqrt_price_at_tick(next_tick);
            let target =
                if (a_to_b) {
                    if (sqrt_next < sqrt_price_limit) { sqrt_price_limit } else { sqrt_next }
                } else {
                    if (sqrt_next > sqrt_price_limit) { sqrt_price_limit } else { sqrt_next }
                };

            let (step_in, step_out, next_sqrt_price, step_fee) =
                clmm_swap_math::compute_swap_step(
                    sqrt_start,
                    target,
                    pool.liquidity,
                    remaining,
                    pool.fee_rate,
                    a_to_b,
                    exact_in
                );
            pool.sqrt_price = next_sqrt_price;
            if (exact_in) {
                remaining = remaining - step_in - step_fee;
            } else {
                remaining = remaining - step_out;
            };
            gross_in = gross_in + step_in + step_fee;
            total_out = total_out + step_out;

            // Fees are charged in the input token and spread over the
            // liquidity that was in place for this step, before any crossing.
            let growth = clmm_position::fee_growth_delta(step_fee, pool.liquidity);
            if (a_to_b) {
                pool.fee_growth_global_a = wrapping_add(pool.fee_growth_global_a, growth);
            } else {
                pool.fee_growth_global_b = wrapping_add(pool.fee_growth_global_b, growth);
            };

            if (next_sqrt_price == sqrt_next) {
                if (initialized) {
                    let net = clmm_tick::cross(
                        &mut pool.ticks,
                        next_tick,
                        pool.fee_growth_global_a,
                        pool.fee_growth_global_b
                    );
                    let signed = if (a_to_b) { -net } else { net };
                    pool.liquidity = clmm_liquidity_math::add_delta(pool.liquidity, signed);
                    crossings = crossings + 1;
                };
                // Stepping past the tick even when it holds nothing is what
                // guarantees the loop makes progress.
                pool.tick = if (a_to_b) { next_tick - 1 } else { next_tick };
            } else if (next_sqrt_price != sqrt_start) {
                pool.tick = clmm_tick_math::get_tick_at_sqrt_price(next_sqrt_price);
            };
        };
        pool.last_swap_crossings = crossings;
        (gross_in, total_out)
    }

    public entry fun swap_exact_in(
        trader: &signer,
        pool_id: address,
        a_to_b: bool,
        amount_in: u64,
        sqrt_price_limit: u128
    ) acquires Pool {
        swap(trader, pool_id, a_to_b, amount_in, true, sqrt_price_limit);
    }

    public entry fun swap_exact_out(
        trader: &signer,
        pool_id: address,
        a_to_b: bool,
        amount_out: u64,
        sqrt_price_limit: u128
    ) acquires Pool {
        swap(trader, pool_id, a_to_b, amount_out, false, sqrt_price_limit);
    }

    fun swap(
        trader: &signer,
        pool_id: address,
        a_to_b: bool,
        amount: u64,
        exact_in: bool,
        sqrt_price_limit: u128
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let trader_address = signer::address_of(trader);
        let pool = borrow_global_mut<Pool>(pool_id);
        let (gross_in, amount_out) =
            run_swap(pool, a_to_b, amount, exact_in, sqrt_price_limit);
        assert!(gross_in != 0 && amount_out != 0, EEMPTY_SWAP);
        let (vault_in, vault_out, token_in) =
            if (a_to_b) {
                (pool.vault_a, pool.vault_b, pool.token_a)
            } else {
                (pool.vault_b, pool.vault_a, pool.token_b)
            };
        let pool_signer = object::generate_signer_for_extending(&pool.extend_ref);
        pay_in(trader, vault_in, token_in, gross_in);
        pay_out(&pool_signer, vault_out, trader_address, amount_out);
    }

    //
    // Benchmark harness surface.
    //

    /// Brings a fresh account to the state the benchmark mix assumes: funded
    /// on both sides of the pair and holding one position.
    public entry fun bench_onboard(
        owner: &signer,
        pool_id: address,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
        fund_amount: u64
    ) acquires Pool {
        faucet_pair(owner, pool_id, fund_amount, fund_amount);
        mint(owner, pool_id, tick_lower, tick_upper, liquidity);
    }

    // The configuration the harness is expected to run with, kept here so the
    // package's own tests hold it to them.
    #[view]
    public fun bench_swap_size(): u64 {
        BENCH_SWAP_SIZE
    }

    #[view]
    public fun bench_backstop_tick(): i32 {
        BENCH_BACKSTOP_TICK
    }

    #[view]
    public fun bench_tick_density(): u32 {
        BENCH_TICK_DENSITY
    }

    /// Faucet the input side, then swap it all the way to the price bound.
    ///
    /// Fauceting first means a long run of swaps in one direction cannot
    /// exhaust the trader, and taking the route's swap rather than `swap`
    /// means a pool with nothing left to give this way is a no-op instead of
    /// an abort. The harness panics on an abort, whatever caused it.
    public entry fun bench_swap_in(
        trader: &signer, pool_id: address, a_to_b: bool, amount_in: u64
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let a_to_b = corrected_direction(pool_id, a_to_b);
        if (a_to_b) {
            faucet_pair(trader, pool_id, amount_in, 0);
        } else {
            faucet_pair(trader, pool_id, 0, amount_in);
        };
        swap_to_the_bound(trader, pool_id, a_to_b, amount_in);
    }

    /// The caller's direction, taken as a hint. A price that has left the
    /// seeded band trades back toward it instead of drifting further.
    ///
    /// This is what an arbitrageur does to a real pool, and it is also what
    /// keeps a long run measuring anything: unchecked, the price random-walks
    /// off the seeded lattice after a handful of trades and every swap after
    /// that crosses only the backstop.
    fun corrected_direction(pool_id: address, hint: bool): bool acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        if (pool.band_lower == pool.band_upper) {
            // An unseeded pool has no band to come back to.
            hint
        } else if (pool.tick > pool.band_upper) {
            true
        } else if (pool.tick < pool.band_lower) {
            false
        } else {
            hint
        }
    }

    /// Mint, burn the same liquidity back, then collect. The position ends
    /// where it started, so the mix can run indefinitely, but it walks
    /// `modify_position` twice and touches every tick in the range.
    public entry fun bench_rebalance(
        owner: &signer,
        pool_id: address,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
        fund_amount: u64
    ) acquires Pool {
        faucet_pair(owner, pool_id, fund_amount, fund_amount);
        mint(owner, pool_id, tick_lower, tick_upper, liquidity);
        burn(owner, pool_id, tick_lower, tick_upper, liquidity);
        collect(
            owner,
            pool_id,
            tick_lower,
            tick_upper,
            (MAX_U64 as u64),
            (MAX_U64 as u64)
        );
    }

    /// Route `amount_in` through the pools named by `path`, each hop selling
    /// whatever the one before it paid out.
    public entry fun bench_swap_multi_hop(
        user: &signer, path: vector<u64>, amount_in: u64
    ) acquires Pool, PoolRegistry {
        route(user, path, amount_in);
    }

    /// The route itself, reporting what every hop it took spent and received
    /// as a flat `[in_0, out_0, in_1, out_1, ..]`.
    ///
    /// A route is best-effort: an empty path, an unregistered pool, a hop that
    /// does not hold the token being carried, or an amount the pool cannot
    /// absorb ends it early rather than failing the transaction.
    public fun route(
        user: &signer, path: vector<u64>, amount_in: u64
    ): vector<u64> acquires Pool, PoolRegistry {
        let legs = vector::empty<u64>();
        let hops = vector::length(&path);
        if (hops == 0 || amount_in == 0 || !exists<PoolRegistry>(@bench)) {
            return legs
        };
        let pools = borrow_global<PoolRegistry>(@bench).pools;
        let count = vector::length(&pools);
        if (count == 0) {
            return legs
        };

        let first = *vector::borrow(&pools, *vector::borrow(&path, 0) % count);
        let second =
            if (hops > 1) {
                *vector::borrow(&pools, *vector::borrow(&path, 1) % count)
            } else { first };
        let token = opening_token(first, second);
        clmm_assets::faucet(token, signer::address_of(user), amount_in);

        let carried = amount_in;
        let i = 0;
        while (i < hops) {
            let pool_id = *vector::borrow(&pools, *vector::borrow(&path, i) % count);
            let (token_out, spent, received) = hop(user, pool_id, token, carried);
            if (received == 0) {
                break
            };
            vector::push_back(&mut legs, spent);
            vector::push_back(&mut legs, received);
            token = token_out;
            carried = received;
            i = i + 1;
        };
        legs
    }

    /// The token a route opens by selling: whichever of `first`'s pair the hop
    /// after it does not also hold.
    ///
    /// A path walking up the chain and one walking back down it have to start
    /// on opposite sides, or the second hop is handed a token it cannot take.
    fun opening_token(first: address, second: address): Object<Metadata> acquires Pool {
        let pool = borrow_global<Pool>(first);
        let (token_a, token_b) = (pool.token_a, pool.token_b);
        if (first == second || !exists<Pool>(second)) {
            return token_a
        };
        let next = borrow_global<Pool>(second);
        if (token_a == next.token_a || token_a == next.token_b) {
            token_b
        } else {
            token_a
        }
    }

    /// One leg of a route: sell `amount` of `token_in` into `pool_id` and
    /// report the token paid out along with the amounts in and out.
    fun hop(
        user: &signer, pool_id: address, token_in: Object<Metadata>, amount: u64
    ): (Object<Metadata>, u64, u64) acquires Pool {
        if (!exists<Pool>(pool_id)) {
            return (token_in, 0, 0)
        };
        let (routable, a_to_b, token_out, reserve) = {
            let pool = borrow_global<Pool>(pool_id);
            let a_to_b = token_in == pool.token_a;
            (
                a_to_b || token_in == pool.token_b,
                a_to_b,
                if (a_to_b) { pool.token_b } else { pool.token_a },
                if (a_to_b) {
                    fungible_asset::balance(pool.vault_a)
                } else {
                    fungible_asset::balance(pool.vault_b)
                }
            )
        };
        if (!routable) {
            return (token_in, 0, 0)
        };
        let cap = reserve / HOP_RESERVE_FRACTION;
        let spent = if (amount > cap) { cap } else { amount };
        if (spent == 0) {
            return (token_in, 0, 0)
        };
        (token_out, spent, swap_to_the_bound(user, pool_id, a_to_b, spent))
    }

    /// Swap toward the price bound and return whatever came out, without
    /// `swap`'s emptiness check. Every benchmark entry goes through here, so a
    /// partial fill or nothing at all is a result rather than an abort.
    ///
    /// A limit has to sit strictly inside the bounds, so step one off them.
    fun swap_to_the_bound(
        trader: &signer, pool_id: address, a_to_b: bool, amount_in: u64
    ): u64 acquires Pool {
        let trader_address = signer::address_of(trader);
        let limit =
            if (a_to_b) clmm_tick_math::min_sqrt_price() + 1
            else clmm_tick_math::max_sqrt_price() - 1;
        let pool = borrow_global_mut<Pool>(pool_id);
        // A pool already resting against the bound has no room left this way.
        if (a_to_b && pool.sqrt_price <= limit) {
            return 0
        };
        if (!a_to_b && pool.sqrt_price >= limit) {
            return 0
        };
        let (gross_in, amount_out) = run_swap(pool, a_to_b, amount_in, true, limit);
        let (vault_in, vault_out, token_in) =
            if (a_to_b) {
                (pool.vault_a, pool.vault_b, pool.token_a)
            } else {
                (pool.vault_b, pool.vault_a, pool.token_b)
            };
        let pool_signer = object::generate_signer_for_extending(&pool.extend_ref);
        pay_in(trader, vault_in, token_in, gross_in);
        pay_out(&pool_signer, vault_out, trader_address, amount_out);
        amount_out
    }

    fun faucet_pair(
        owner: &signer, pool_id: address, amount_a: u64, amount_b: u64
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let owner_address = signer::address_of(owner);
        let (token_a, token_b) = {
            let pool = borrow_global<Pool>(pool_id);
            (pool.token_a, pool.token_b)
        };
        if (amount_a != 0) {
            clmm_assets::faucet(token_a, owner_address, amount_a);
        };
        if (amount_b != 0) {
            clmm_assets::faucet(token_b, owner_address, amount_b);
        }
    }

    //
    // Seeding.
    //

    /// Tile `n_positions` ranges outward from the current price on a lattice
    /// whose step `tick_density` sets, and record the density and the span
    /// they reached on the pool.
    ///
    /// Tiling rather than jittering is what makes the knob mean something: the
    /// boundaries a swap meets are exactly one step apart, so a trade of a
    /// given size crosses a number of ticks that the density divides.
    public entry fun seed_positions(
        admin: &signer, pool_id: address, n_positions: u64, tick_density: u32, seed: u64
    ) acquires Pool {
        assert!(exists<Pool>(pool_id), ENO_POOL);
        let admin_address = signer::address_of(admin);
        let (spacing, center_tick) = {
            let pool = borrow_global<Pool>(pool_id);
            assert!(pool.admin == admin_address, ENOT_POOL_ADMIN);
            (pool.tick_spacing, pool.tick)
        };
        let stride = density_stride(spacing, tick_density) as i32;
        let center = align(center_tick, stride);

        // Reduced first so a large seed cannot overflow the multiply.
        let state = seed % LCG_MOD;
        let band_lower = center;
        let band_upper = center;
        let i = 0;
        while (i < n_positions) {
            // Ranges alternate above and below the price, so a swap in either
            // direction meets a boundary every `stride` ticks.
            let step = ((i / 2 + 1) as i32);
            let (lower, upper) =
                if (i % 2 == 0) {
                    (center + (step - 1) * stride, center + step * stride)
                } else {
                    (center - step * stride, center - (step - 1) * stride)
                };
            let (lower, upper) = clamp_range(lower, upper, stride);
            if (lower < upper) {
                state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
                let liquidity = 1000000000000u128 + (state as u128) * 1000000u128;
                mint(admin, pool_id, lower, upper, liquidity);
                if (lower < band_lower) {
                    band_lower = lower;
                };
                if (upper > band_upper) {
                    band_upper = upper;
                };
            };
            i = i + 1;
        };
        let pool = borrow_global_mut<Pool>(pool_id);
        pool.tick_density = tick_density;
        pool.band_lower = band_lower;
        pool.band_upper = band_upper;
    }

    /// Ticks between two adjacent seeded boundaries. A density of zero or one
    /// spreads them the full window apart and anything at or above the window
    /// packs them onto the pool's own spacing, which is as tight as they go.
    public fun density_stride(tick_spacing: u32, tick_density: u32): u32 {
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        let spacings =
            if (tick_density == 0) { DENSITY_SPACINGS }
            else if (tick_density >= DENSITY_SPACINGS) { 1 }
            else { DENSITY_SPACINGS / tick_density };
        spacings * tick_spacing
    }

    /// Global fee growth is read only as a difference, so it wraps rather than
    /// aborting once it fills a `u128`.
    fun wrapping_add(a: u128, b: u128): u128 {
        ((((a as u256) + (b as u256)) & MAX_U128) as u128)
    }

    /// Largest multiple of `spacing` at or below `tick`.
    fun align(tick: i32, spacing: i32): i32 {
        clmm_tick_bitmap::compress(tick, spacing as u32) * spacing
    }

    fun clamp_range(lower: i32, upper: i32, spacing: i32): (i32, i32) {
        let floor = align(clmm_tick_math::min_tick(), spacing);
        if (floor < clmm_tick_math::min_tick()) {
            floor = floor + spacing;
        };
        let ceiling = align(clmm_tick_math::max_tick(), spacing);
        let lower = if (lower < floor) { floor } else { lower };
        let upper = if (upper > ceiling) { ceiling } else { upper };
        (lower, upper)
    }

    //
    // Transfers.
    //

    fun pay_in(
        payer: &signer, vault: Object<FungibleStore>, token: Object<Metadata>, amount: u64
    ) {
        if (amount == 0) {
            return
        };
        fungible_asset::deposit(vault, primary_fungible_store::withdraw(payer, token, amount));
    }

    fun pay_out(
        pool_signer: &signer, vault: Object<FungibleStore>, to: address, amount: u64
    ) {
        if (amount == 0) {
            return
        };
        primary_fungible_store::deposit(to, fungible_asset::withdraw(pool_signer, vault, amount));
    }

    //
    // Views.
    //

    #[view]
    public fun state(pool_id: address): (u128, i32, u128, u128, u128) acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        (
            pool.sqrt_price,
            pool.tick,
            pool.liquidity,
            pool.fee_growth_global_a,
            pool.fee_growth_global_b
        )
    }

    #[view]
    public fun liquidity(pool_id: address): u128 acquires Pool {
        borrow_global<Pool>(pool_id).liquidity
    }

    #[view]
    public fun tick_density(pool_id: address): u32 acquires Pool {
        borrow_global<Pool>(pool_id).tick_density
    }

    // Initialized ticks the most recent swap through this pool crossed, which
    // is the read-set width the density knob moves.
    #[view]
    public fun last_swap_crossings(pool_id: address): u64 acquires Pool {
        borrow_global<Pool>(pool_id).last_swap_crossings
    }

    // The tick span the seeded ranges cover, which is the band a swap is
    // steered back toward. Equal bounds mean the pool was never seeded.
    #[view]
    public fun seeded_band(pool_id: address): (i32, i32) acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        (pool.band_lower, pool.band_upper)
    }

    #[view]
    public fun pool_count(): u64 acquires PoolRegistry {
        if (!exists<PoolRegistry>(@bench)) { 0 }
        else { vector::length(&borrow_global<PoolRegistry>(@bench).pools) }
    }

    #[view]
    public fun pool_at(index: u64): address acquires PoolRegistry {
        *vector::borrow(&borrow_global<PoolRegistry>(@bench).pools, index)
    }

    #[view]
    public fun current_tick(pool_id: address): i32 acquires Pool {
        borrow_global<Pool>(pool_id).tick
    }

    #[view]
    public fun sqrt_price(pool_id: address): u128 acquires Pool {
        borrow_global<Pool>(pool_id).sqrt_price
    }

    #[view]
    public fun vault_balances(pool_id: address): (u64, u64) acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        (fungible_asset::balance(pool.vault_a), fungible_asset::balance(pool.vault_b))
    }

    #[view]
    public fun position_liquidity(
        pool_id: address, owner: address, tick_lower: i32, tick_upper: i32
    ): u128 acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        clmm_position::liquidity(&pool.positions, clmm_position::key(owner, tick_lower, tick_upper))
    }

    #[view]
    public fun position_tokens_owed(
        pool_id: address, owner: address, tick_lower: i32, tick_upper: i32
    ): (u64, u64) acquires Pool {
        let pool = borrow_global<Pool>(pool_id);
        clmm_position::tokens_owed(
            &pool.positions, clmm_position::key(owner, tick_lower, tick_upper)
        )
    }

    #[view]
    public fun tick_is_initialized(pool_id: address, tick: i32): bool acquires Pool {
        clmm_tick::is_initialized(&borrow_global<Pool>(pool_id).ticks, tick)
    }

    #[view]
    public fun tick_liquidity_net(pool_id: address, tick: i32): i128 acquires Pool {
        clmm_tick::liquidity_net(&borrow_global<Pool>(pool_id).ticks, tick)
    }

    #[view]
    public fun tick_liquidity_gross(pool_id: address, tick: i32): u128 acquires Pool {
        clmm_tick::liquidity_gross(&borrow_global<Pool>(pool_id).ticks, tick)
    }
}
