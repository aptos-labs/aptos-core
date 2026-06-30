// V2-only test for the aggregator_v2 natives (u64 and u128).
//
// `aggregator_v2` is declared locally (with the same struct fields and native
// signatures as the framework) to avoid pulling in the Aptos Framework, and
// because the legacy harness installs no `NativeAggregatorContext` to run these
// natives. Entry wrappers take/return primitives only and instantiate the
// generic natives at a concrete integer type.

// RUN: publish
module 0x1::aggregator_v2 {
    struct Aggregator<IntElement> has store, drop {
        value: IntElement,
        max_value: IntElement,
    }

    struct AggregatorSnapshot<IntElement> has store, drop {
        value: IntElement,
    }

    public native fun create_aggregator<IntElement>(max_value: IntElement): Aggregator<IntElement>;
    public native fun create_unbounded_aggregator<IntElement>(): Aggregator<IntElement>;
    public native fun try_add<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool;
    public native fun try_sub<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool;
    native fun is_at_least_impl<IntElement>(self: &Aggregator<IntElement>, min_amount: IntElement): bool;
    public native fun read<IntElement>(self: &Aggregator<IntElement>): IntElement;
    public native fun snapshot<IntElement>(self: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;
    public native fun create_snapshot<IntElement>(value: IntElement): AggregatorSnapshot<IntElement>;
    public native fun read_snapshot<IntElement>(self: &AggregatorSnapshot<IntElement>): IntElement;

    public fun is_at_least<IntElement>(self: &Aggregator<IntElement>, min_amount: IntElement): bool {
        is_at_least_impl(self, min_amount)
    }
}

module 0x1::main {
    use 0x1::aggregator_v2;

    public fun add_read_u64(a: u64, b: u64): u64 {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::try_add(&mut agg, b);
        aggregator_v2::read(&agg)
    }

    // Returns whether the subtraction succeeded (false when b > a).
    public fun sub_ok_u64(a: u64, b: u64): bool {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::try_sub(&mut agg, b)
    }

    public fun at_least_u64(a: u64, m: u64): bool {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::is_at_least(&agg, m)
    }

    public fun add_read_u128(a: u128, b: u128): u128 {
        let agg = aggregator_v2::create_unbounded_aggregator<u128>();
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::try_add(&mut agg, b);
        aggregator_v2::read(&agg)
    }

    // Adds `a` then `b` to an aggregator bounded by `max`; returns whether the
    // second add stayed within `max`.
    public fun bounded_add_u64(max: u64, a: u64, b: u64): bool {
        let agg = aggregator_v2::create_aggregator<u64>(max);
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::try_add(&mut agg, b)
    }

    public fun create_add_read_u128(max: u128, a: u128): u128 {
        let agg = aggregator_v2::create_aggregator<u128>(max);
        aggregator_v2::try_add(&mut agg, a);
        aggregator_v2::read(&agg)
    }

    // Snapshot an aggregator's current value, then read it back.
    public fun snapshot_read_u64(a: u64): u64 {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add(&mut agg, a);
        let snap = aggregator_v2::snapshot(&agg);
        aggregator_v2::read_snapshot(&snap)
    }

    // Create a snapshot directly from a value and read it back.
    public fun create_read_snapshot_u64(v: u64): u64 {
        let snap = aggregator_v2::create_snapshot<u64>(v);
        aggregator_v2::read_snapshot(&snap)
    }

    public fun create_read_snapshot_u128(v: u128): u128 {
        let snap = aggregator_v2::create_snapshot<u128>(v);
        aggregator_v2::read_snapshot(&snap)
    }
}

// RUN: execute 0x1::main::add_read_u64 --args 10, 32
// CHECK-V2: results: 42

// RUN: execute 0x1::main::sub_ok_u64 --args 5, 3
// CHECK-V2: results: true

// RUN: execute 0x1::main::sub_ok_u64 --args 3, 5
// CHECK-V2: results: false

// RUN: execute 0x1::main::at_least_u64 --args 7, 7
// CHECK-V2: results: true

// RUN: execute 0x1::main::at_least_u64 --args 7, 8
// CHECK-V2: results: false

// RUN: execute 0x1::main::add_read_u128 --args 10, 32
// CHECK-V2: results: 42

// 7 + 3 = 10 <= max (10), so the second add fits.
// RUN: execute 0x1::main::bounded_add_u64 --args 10, 7, 3
// CHECK-V2: results: true

// 7 + 5 = 12 > max (10), so the second add is rejected.
// RUN: execute 0x1::main::bounded_add_u64 --args 10, 7, 5
// CHECK-V2: results: false

// RUN: execute 0x1::main::create_add_read_u128 --args 100, 42
// CHECK-V2: results: 42

// RUN: execute 0x1::main::snapshot_read_u64 --args 42
// CHECK-V2: results: 42

// RUN: execute 0x1::main::create_read_snapshot_u64 --args 7
// CHECK-V2: results: 7

// RUN: execute 0x1::main::create_read_snapshot_u128 --args 99
// CHECK-V2: results: 99
