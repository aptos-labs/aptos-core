/// The aggregator front end: the entry points a harness actually submits.
///
/// Every route is a chain of `dexr_backend` hops whose venues and legs come in
/// as type arguments. A hop returns what it managed to trade and the next hop
/// takes that, so a route that fills short keeps going instead of unwinding.
module bench::dexr_router {
    use std::signer;
    use std::vector;
    use bench::dexr_assets;
    use bench::dexr_backend;
    use bench::dexr_markers::{A0, A1, A2, A3, A4, A5, A6, A7};
    use bench::dexr_math;
    use bench::dexr_pool_book;
    use bench::dexr_pool_clmm;
    use bench::dexr_pool_cpmm;
    use bench::dexr_pool_stable;

    /// Only the package address may configure the aggregator.
    const E_NOT_BENCH: u64 = 1;

    /// `map_markers` was handed something other than the eight benchmark
    /// assets.
    const E_BAD_ASSET_COUNT: u64 = 2;

    const N_ASSETS: u64 = 8;

    struct Registry has key {
        assets: vector<address>,
    }

    /// Stand up the marker table and all four pool tables in one transaction,
    /// so pool creation can start on the next one.
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Registry>(@bench)) {
            move_to(admin, Registry { assets: vector::empty() });
        };
        dexr_backend::initialize(admin);
        dexr_pool_cpmm::initialize(admin);
        dexr_pool_stable::initialize(admin);
        dexr_pool_clmm::initialize(admin);
        dexr_pool_book::initialize(admin);
    }

    /// Bind the eight asset markers to the eight assets, in order.
    public entry fun map_markers(
        admin: &signer, assets: vector<address>
    ) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(vector::length(&assets) == N_ASSETS, E_BAD_ASSET_COUNT);
        dexr_backend::set_fallback(admin, *vector::borrow(&assets, 0));
        dexr_backend::set_marker<A0>(admin, *vector::borrow(&assets, 0));
        dexr_backend::set_marker<A1>(admin, *vector::borrow(&assets, 1));
        dexr_backend::set_marker<A2>(admin, *vector::borrow(&assets, 2));
        dexr_backend::set_marker<A3>(admin, *vector::borrow(&assets, 3));
        dexr_backend::set_marker<A4>(admin, *vector::borrow(&assets, 4));
        dexr_backend::set_marker<A5>(admin, *vector::borrow(&assets, 5));
        dexr_backend::set_marker<A6>(admin, *vector::borrow(&assets, 6));
        dexr_backend::set_marker<A7>(admin, *vector::borrow(&assets, 7));
        borrow_global_mut<Registry>(@bench).assets = assets;
    }

    public fun assets(): vector<address> acquires Registry {
        if (!exists<Registry>(@bench)) { vector::empty() }
        else { borrow_global<Registry>(@bench).assets }
    }

    /// Fund a trader in every asset, so no route has to care which leg the
    /// account happens to be holding.
    public entry fun bench_onboard(user: &signer, amount: u64) acquires Registry {
        if (!exists<Registry>(@bench)) {
            return
        };
        let owner = signer::address_of(user);
        let assets = &borrow_global<Registry>(@bench).assets;
        let n = vector::length(assets);
        let i = 0;
        while (i < n) {
            let asset = *vector::borrow(assets, i);
            dexr_assets::faucet(dexr_assets::metadata_at(asset), owner, amount);
            i = i + 1;
        };
    }

    public entry fun bench_route1<B0, X0, X1, M0>(
        user: &signer, pool_id: u64, amount_in: u64
    ) {
        let base = pool_id + dexr_backend::mode_bits<M0>();
        let _ = dexr_backend::swap<B0, X0, X1>(user, base, amount_in);
    }

    public entry fun bench_route2<B0, B1, X0, X1, X2, M0, M1, M2>(
        user: &signer, pool_id: u64, amount_in: u64
    ) {
        let base = pool_id + dexr_backend::mode_bits3<M0, M1, M2>();
        let out = dexr_backend::swap<B0, X0, X1>(user, base, amount_in);
        let _ = dexr_backend::swap<B1, X1, X2>(user, base + 1, out);
    }

    public entry fun bench_route3<
        B0, B1, B2,
        X0, X1, X2, X3,
        M0, M1, M2, M3, M4, M5, M6, M7, M8,
    >(user: &signer, pool_id: u64, amount_in: u64) {
        let base = pool_id
            + dexr_backend::mode_bits3<M0, M1, M2>()
            + dexr_backend::mode_bits3<M3, M4, M5>()
            + dexr_backend::mode_bits3<M6, M7, M8>();
        let out = dexr_backend::swap<B0, X0, X1>(user, base, amount_in);
        let out = dexr_backend::swap<B1, X1, X2>(user, base + 1, out);
        let _ = dexr_backend::swap<B2, X2, X3>(user, base + 2, out);
    }

    /// The thirty-two argument shape. The verifier caps a generic
    /// instantiation at thirty-two type arguments, so this signature sits
    /// exactly on the limit and one more would fail to verify.
    public entry fun bench_route5<
        B0, B1, B2, B3, B4,
        X0, X1, X2, X3, X4, X5,
        M0, M1, M2, M3, M4, M5, M6,
        M7, M8, M9, M10, M11, M12, M13,
        M14, M15, M16, M17, M18, M19, M20,
    >(user: &signer, pool_id: u64, amount_in: u64) {
        let base = pool_id
            + dexr_backend::mode_bits7<M0, M1, M2, M3, M4, M5, M6>()
            + dexr_backend::mode_bits7<M7, M8, M9, M10, M11, M12, M13>()
            + dexr_backend::mode_bits7<M14, M15, M16, M17, M18, M19, M20>();
        let out = dexr_backend::swap<B0, X0, X1>(user, base, amount_in);
        let out = dexr_backend::swap<B1, X1, X2>(user, base + 1, out);
        let out = dexr_backend::swap<B2, X2, X3>(user, base + 2, out);
        let out = dexr_backend::swap<B3, X3, X4>(user, base + 3, out);
        let _ = dexr_backend::swap<B4, X4, X5>(user, base + 4, out);
    }

    /// Split the first leg across two venues, then merge the two halves back
    /// through two more. `split_bps` is the share the first venue takes.
    public entry fun bench_split<
        B0, B1, B2, B3,
        X0, X1, X2,
        M0, M1, M2, M3, M4, M5, M6, M7, M8,
    >(user: &signer, pool_id: u64, amount_in: u64, split_bps: u64) {
        let base = pool_id
            + dexr_backend::mode_bits3<M0, M1, M2>()
            + dexr_backend::mode_bits3<M3, M4, M5>()
            + dexr_backend::mode_bits3<M6, M7, M8>();
        let first = dexr_math::mul_div(
            amount_in, dexr_math::min(split_bps, dexr_math::bps()), dexr_math::bps()
        );
        let second = amount_in - first;
        let a = dexr_backend::swap<B0, X0, X1>(user, base, first);
        let b = dexr_backend::swap<B1, X0, X1>(user, base + 1, second);
        let c = dexr_backend::swap<B2, X1, X2>(user, base + 2, a);
        let d = dexr_backend::swap<B3, X1, X2>(user, base + 3, b);
        let _ = c + d;
    }

    /// Price a three-hop route without trading it. Same dispatch as
    /// `bench_route3` and the same instantiation cost, with no writes.
    public entry fun bench_quote<
        B0, B1, B2,
        X0, X1, X2, X3,
        M0, M1, M2, M3, M4, M5, M6, M7, M8,
    >(_user: &signer, pool_id: u64, amount_in: u64) {
        let base = pool_id
            + dexr_backend::mode_bits3<M0, M1, M2>()
            + dexr_backend::mode_bits3<M3, M4, M5>()
            + dexr_backend::mode_bits3<M6, M7, M8>();
        let out = dexr_backend::quote<B0, X0, X1>(base, amount_in);
        let out = dexr_backend::quote<B1, X1, X2>(base + 1, out);
        let _ = dexr_backend::quote<B2, X2, X3>(base + 2, out);
    }

    public entry fun bench_rebalance(
        _user: &signer, backend: u8, pool_id: u64, amount: u64
    ) {
        dexr_backend::rebalance(backend, pool_id, amount);
    }
}
