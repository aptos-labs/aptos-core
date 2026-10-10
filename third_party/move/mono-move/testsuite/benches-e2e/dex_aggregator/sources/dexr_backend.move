/// The generic trampoline every hop goes through.
///
/// A route names its venue and its two legs as type arguments. This module
/// turns those types back into a backend selector and two asset addresses, then
/// calls the concrete pool. That indirection is the point of the workload: the
/// call chain is monomorphized per instantiation even though the code under it
/// is shared.
module bench::dexr_backend {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};
    use bench::dexr_markers::{Book, Clmm, Cpmm, Stable};
    use bench::dexr_pool_book;
    use bench::dexr_pool_clmm;
    use bench::dexr_pool_cpmm;
    use bench::dexr_pool_stable;

    /// Only the package address may bind markers.
    const E_NOT_BENCH: u64 = 1;

    /// Backend selectors, in the order `tag` returns them.
    const TAG_CPMM: u8 = 0;
    const TAG_STABLE: u8 = 1;
    const TAG_CLMM: u8 = 2;
    const TAG_BOOK: u8 = 3;

    /// How many distinct pool offsets a mode marker can select.
    const MODE_SPAN: u64 = 4;

    struct Markers has key {
        map: Table<TypeInfo, address>,
        /// Where a marker with no binding resolves.
        fallback: address,
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Markers>(@bench)) {
            move_to(admin, Markers { map: table::new(), fallback: @0x0 });
        };
    }

    public fun set_fallback(admin: &signer, asset: address) acquires Markers {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        borrow_global_mut<Markers>(@bench).fallback = asset;
    }

    public fun set_marker<T>(admin: &signer, asset: address) acquires Markers {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let markers = borrow_global_mut<Markers>(@bench);
        let info = type_info::type_of<T>();
        if (table::contains(&markers.map, info)) {
            *table::borrow_mut(&mut markers.map, info) = asset;
        } else {
            table::add(&mut markers.map, info, asset);
        };
    }

    /// The asset a leg marker stands for. A marker nobody bound resolves to the
    /// fallback, so a route may name any type and still trade.
    public fun resolve<T>(): address acquires Markers {
        if (!exists<Markers>(@bench)) {
            return @0x0
        };
        let markers = borrow_global<Markers>(@bench);
        let info = type_info::type_of<T>();
        if (table::contains(&markers.map, info)) {
            *table::borrow(&markers.map, info)
        } else {
            markers.fallback
        }
    }

    /// The backend a venue marker selects. Anything but the four backend tags
    /// takes the constant product path.
    public fun tag<B>(): u8 {
        let info = type_info::type_of<B>();
        if (info == type_info::type_of<Cpmm>()) { TAG_CPMM }
        else if (info == type_info::type_of<Stable>()) { TAG_STABLE }
        else if (info == type_info::type_of<Clmm>()) { TAG_CLMM }
        else if (info == type_info::type_of<Book>()) { TAG_BOOK }
        else { TAG_CPMM }
    }

    /// The pool offset a mode marker selects, taken off the last byte of its
    /// name so two different markers usually land on two different pools.
    public fun mode_bits<M>(): u64 {
        let name = type_info::struct_name(&type_info::type_of<M>());
        let len = vector::length(&name);
        if (len == 0) { 0 } else { (*vector::borrow(&name, len - 1) as u64) % MODE_SPAN }
    }

    /// Three markers' worth of offset. Route bodies batch their mode markers
    /// through these so a five-hop signature stays readable.
    public fun mode_bits3<M0, M1, M2>(): u64 {
        mode_bits<M0>() + mode_bits<M1>() + mode_bits<M2>()
    }

    public fun mode_bits7<M0, M1, M2, M3, M4, M5, M6>(): u64 {
        mode_bits3<M0, M1, M2>() + mode_bits3<M3, M4, M5>() + mode_bits<M6>()
    }

    /// Trade `amount_in` of `X` for `Y` on venue `B`, pool `pool_id`. Returns
    /// what the pool gave back, which may be less than a caller wanted and is
    /// zero when the pool has nothing to give.
    public fun swap<B, X, Y>(
        user: &signer, pool_id: u64, amount_in: u64
    ): u64 acquires Markers {
        let asset_in = resolve<X>();
        let asset_out = resolve<Y>();
        let tag = tag<B>();
        if (tag == TAG_CPMM) {
            dexr_pool_cpmm::swap(user, pool_id, asset_in, asset_out, amount_in)
        } else if (tag == TAG_STABLE) {
            dexr_pool_stable::swap(user, pool_id, asset_in, asset_out, amount_in)
        } else if (tag == TAG_CLMM) {
            dexr_pool_clmm::swap(user, pool_id, asset_in, asset_out, amount_in)
        } else {
            dexr_pool_book::swap(user, pool_id, asset_in, asset_out, amount_in)
        }
    }

    public fun quote<B, X, Y>(pool_id: u64, amount_in: u64): u64 acquires Markers {
        let asset_in = resolve<X>();
        let asset_out = resolve<Y>();
        let tag = tag<B>();
        if (tag == TAG_CPMM) {
            dexr_pool_cpmm::quote(pool_id, asset_in, asset_out, amount_in)
        } else if (tag == TAG_STABLE) {
            dexr_pool_stable::quote(pool_id, asset_in, asset_out, amount_in)
        } else if (tag == TAG_CLMM) {
            dexr_pool_clmm::quote(pool_id, asset_in, asset_out, amount_in)
        } else {
            dexr_pool_book::quote(pool_id, asset_in, asset_out, amount_in)
        }
    }

    /// Top a pool up on the backend `backend` selects, wrapping the selector so
    /// any byte names a real backend.
    public fun rebalance(backend: u8, pool_id: u64, amount: u64) {
        let tag = backend % 4;
        if (tag == TAG_CPMM) {
            dexr_pool_cpmm::rebalance(pool_id, amount);
        } else if (tag == TAG_STABLE) {
            dexr_pool_stable::rebalance(pool_id, amount);
        } else if (tag == TAG_CLMM) {
            dexr_pool_clmm::rebalance(pool_id, amount);
        } else {
            dexr_pool_book::rebalance(pool_id, amount);
        };
    }

    /// Pools the backend `backend` selects currently holds.
    public fun n_pools(backend: u8): u64 {
        let tag = backend % 4;
        if (tag == TAG_CPMM) { dexr_pool_cpmm::n_pools() }
        else if (tag == TAG_STABLE) { dexr_pool_stable::n_pools() }
        else if (tag == TAG_CLMM) { dexr_pool_clmm::n_pools() }
        else { dexr_pool_book::n_pools() }
    }
}
