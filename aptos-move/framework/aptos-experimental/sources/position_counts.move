/// Per-exchange position-count ceiling, enforced atomically via AggregatorV2.
///
/// One `PositionCounters` resource lives at `@aptos_experimental`, holding a
/// table keyed by `exchange`. `native_position::register()` allocates a
/// counter with `max = initial_max` when an exchange first registers;
/// `create_position` / `remove_position` bump / decrement it; governance can
/// tune `max` via `update_ceiling`.
///
/// Delayed-field semantics on `AggregatorV2<u64>` mean concurrent
/// `try_add` / `sub` calls on the same counter don't conflict in Block-STM,
/// as long as the bound isn't hit.
module aptos_experimental::position_counts {
    use std::error;
    use std::signer;

    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};

    friend aptos_experimental::native_position;

    /// PositionCounters resource has not been initialized yet.
    const ENOT_INITIALIZED: u64 = 2;
    /// No counter has been allocated for this exchange.
    const ECOUNTER_NOT_FOUND: u64 = 3;
    /// Counter already allocated for this exchange.
    const ECOUNTER_ALREADY_ALLOCATED: u64 = 4;
    /// try_add would exceed the configured ceiling.
    const EPOSITION_LIMIT: u64 = 5;
    /// try_sub would underflow.
    const ECOUNTER_UNDERFLOW: u64 = 6;

    struct PositionCounters has key {
        counts: Table<address, Aggregator<u64>>,
    }

    /// Runs once when the module is published at `@aptos_experimental`.
    fun init_module(experimental: &signer) {
        if (!exists<PositionCounters>(signer::address_of(experimental))) {
            move_to(experimental, PositionCounters { counts: table::new() });
        };
    }

    /// Genesis hook: vm-genesis calls this explicitly after publishing
    /// the framework since it doesn't auto-invoke `init_module` for
    /// release bundle packages. Called with a signer for
    /// `@aptos_experimental` (0x7). Idempotent.
    public fun initialize_for_genesis(experimental: &signer) {
        assert!(
            signer::address_of(experimental) == @aptos_experimental,
            error::permission_denied(ENOT_INITIALIZED),
        );
        if (!exists<PositionCounters>(signer::address_of(experimental))) {
            move_to(experimental, PositionCounters { counts: table::new() });
        };
    }

    /// Allocate a counter for a newly registered exchange.
    ///
    /// Called by `native_position::register()` when the exchange's signer
    /// first registers. If a counter already exists for `exchange`, this
    /// aborts — callers must check for existence first when implementing
    /// idempotent register semantics.
    public(friend) fun allocate_counter(exchange: address, initial_max: u64) acquires PositionCounters {
        assert!(
            exists<PositionCounters>(@aptos_experimental),
            error::not_found(ENOT_INITIALIZED),
        );
        let counters = &mut borrow_global_mut<PositionCounters>(@aptos_experimental).counts;
        assert!(
            !table::contains(counters, exchange),
            error::already_exists(ECOUNTER_ALREADY_ALLOCATED),
        );
        table::add(
            counters,
            exchange,
            aggregator_v2::create_aggregator(initial_max),
        );
    }

    public(friend) fun counter_exists(exchange: address): bool acquires PositionCounters {
        if (!exists<PositionCounters>(@aptos_experimental)) {
            return false
        };
        let counters = &borrow_global<PositionCounters>(@aptos_experimental).counts;
        table::contains(counters, exchange)
    }

    /// Try to increment the counter for `exchange`. Aborts
    /// `EPOSITION_LIMIT` if it would exceed the configured ceiling.
    public(friend) fun try_add(exchange: address, delta: u64) acquires PositionCounters {
        let counters = &mut borrow_global_mut<PositionCounters>(@aptos_experimental).counts;
        assert!(
            table::contains(counters, exchange),
            error::not_found(ECOUNTER_NOT_FOUND),
        );
        let agg = table::borrow_mut(counters, exchange);
        assert!(
            aggregator_v2::try_add(agg, delta),
            error::out_of_range(EPOSITION_LIMIT),
        );
    }

    /// Decrement the counter for `exchange`. Aborts `ECOUNTER_UNDERFLOW`
    /// if the counter is already zero.
    public(friend) fun sub(exchange: address, delta: u64) acquires PositionCounters {
        let counters = &mut borrow_global_mut<PositionCounters>(@aptos_experimental).counts;
        assert!(
            table::contains(counters, exchange),
            error::not_found(ECOUNTER_NOT_FOUND),
        );
        let agg = table::borrow_mut(counters, exchange);
        assert!(
            aggregator_v2::try_sub(agg, delta),
            error::out_of_range(ECOUNTER_UNDERFLOW),
        );
    }

    /// Governance-only: adjust the ceiling for an existing exchange's
    /// counter. Typical use: raise before a large migration, lower to
    /// squeeze a misbehaving tenant. Replaces the old aggregator with a
    /// fresh one bounded at `new_max` and carrying the current value
    /// (clamped to `new_max` if it would overflow).
    public fun update_ceiling(
        framework: &signer,
        exchange: address,
        new_max: u64,
    ) acquires PositionCounters {
        system_addresses::assert_aptos_framework(framework);
        let counters = &mut borrow_global_mut<PositionCounters>(@aptos_experimental).counts;
        assert!(
            table::contains(counters, exchange),
            error::not_found(ECOUNTER_NOT_FOUND),
        );
        let old_agg = table::remove(counters, exchange);
        let current = aggregator_v2::read(&old_agg);
        let target = if (current > new_max) { new_max } else { current };
        let replacement = aggregator_v2::create_aggregator(new_max);
        if (target > 0) {
            // Succeeds because target <= new_max.
            aggregator_v2::try_add(&mut replacement, target);
        };
        table::add(counters, exchange, replacement);
    }

    // =====================================================================
    // Tests
    // =====================================================================

    // Happy-path: allocate, increment to ceiling, decrement back to zero.
    #[test(experimental = @aptos_experimental)]
    fun test_counter_basic(experimental: &signer) acquires PositionCounters {
        initialize_for_genesis(experimental);
        allocate_counter(@0xa1, 3);
        try_add(@0xa1, 1);
        try_add(@0xa1, 1);
        try_add(@0xa1, 1);
        sub(@0xa1, 1);
        sub(@0xa1, 1);
        sub(@0xa1, 1);
    }

    // A `try_add` past the configured ceiling must abort EPOSITION_LIMIT
    // (= 5, wrapped by error::out_of_range → 0x20005). Regression guard
    // for the `create_position` counter-bumping wiring — if
    // `position_counts::try_add` ever stops asserting, the per-exchange
    // position cap silently goes away.
    #[test(experimental = @aptos_experimental)]
    #[expected_failure(abort_code = 0x20005, location = Self)]
    fun test_try_add_aborts_at_ceiling(
        experimental: &signer
    ) acquires PositionCounters {
        initialize_for_genesis(experimental);
        allocate_counter(@0xa7, 2);
        try_add(@0xa7, 1);
        try_add(@0xa7, 1);
        // Third increment crosses the ceiling.
        try_add(@0xa7, 1);
    }

    // Decrementing past zero must abort ECOUNTER_UNDERFLOW (= 6 →
    // 0x20006). Regression guard for the `remove_position` counter
    // wiring — if caller-side bookkeeping drifts (remove called
    // without a matching create), the abort here is the safety net.
    #[test(experimental = @aptos_experimental)]
    #[expected_failure(abort_code = 0x20006, location = Self)]
    fun test_sub_aborts_on_underflow(
        experimental: &signer
    ) acquires PositionCounters {
        initialize_for_genesis(experimental);
        allocate_counter(@0xa7, 10);
        sub(@0xa7, 1);
    }

    // try_add on a non-existent counter aborts ECOUNTER_NOT_FOUND
    // (= 3, error::not_found → 0x60003). Defense in depth: caller is
    // expected to have called `allocate_counter` first via
    // `native_position::register()`.
    #[test(experimental = @aptos_experimental)]
    #[expected_failure(abort_code = 0x60003, location = Self)]
    fun test_try_add_unknown_exchange(
        experimental: &signer
    ) acquires PositionCounters {
        initialize_for_genesis(experimental);
        try_add(@0xdead, 1);
    }

    // update_ceiling must clamp the counter when shrinking below
    // the current value, AND the new try_add at the boundary aborts.
    #[test(framework = @aptos_framework, experimental = @aptos_experimental)]
    #[expected_failure(abort_code = 0x20005, location = Self)]
    fun test_update_ceiling_clamps_and_caps(
        framework: &signer,
        experimental: &signer,
    ) acquires PositionCounters {
        initialize_for_genesis(experimental);
        allocate_counter(@0xa1, 10);
        try_add(@0xa1, 7);
        // Shrink to 3 — new aggregator carries min(current, new_max).
        update_ceiling(framework, @0xa1, 3);
        // Counter is now at 3 (clamped); next try_add aborts.
        try_add(@0xa1, 1);
    }
}
