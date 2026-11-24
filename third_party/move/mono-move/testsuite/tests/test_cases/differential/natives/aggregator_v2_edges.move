// aggregator_v2 edge cases. Checked on MonoMove only: the V1 side of the harness has no
// aggregator support.
// - u128 `try_sub`, `is_at_least` and `snapshot` on values above u64::MAX, where a 64-bit
//   read or write would give a different result.
// - `try_add` at the integer maximum of an unbounded aggregator overflows, returns false and
//   leaves the value unchanged; the same add one below the maximum succeeds.
// - `read` of an `Aggregator` whose value exceeds its max is an invariant violation. Only the
//   test's own declaration of the module can build that value, by packing it directly.

// RUN: publish
module 0x1::aggregator_v2 {
    struct Aggregator<IntElement> has store, drop {
        value: IntElement,
        max_value: IntElement,
    }

    struct AggregatorSnapshot<IntElement> has store, drop {
        value: IntElement,
    }

    public native fun create_unbounded_aggregator<IntElement>(): Aggregator<IntElement>;
    public native fun try_add<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool;
    public native fun try_sub<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool;
    native fun is_at_least_impl<IntElement>(self: &Aggregator<IntElement>, min_amount: IntElement): bool;
    public native fun read<IntElement>(self: &Aggregator<IntElement>): IntElement;
    public native fun snapshot<IntElement>(self: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;
    public native fun read_snapshot<IntElement>(self: &AggregatorSnapshot<IntElement>): IntElement;

    public fun is_at_least<IntElement>(self: &Aggregator<IntElement>, min_amount: IntElement): bool {
        is_at_least_impl(self, min_amount)
    }

    public fun read_packed_u64(value: u64, max_value: u64): u64 {
        read(&Aggregator { value, max_value })
    }

    public fun read_packed_u128(value: u128, max_value: u128): u128 {
        read(&Aggregator { value, max_value })
    }
}

module 0x1::main {
    use 0x1::aggregator_v2;

    const TWO_POW_64: u128 = 18446744073709551616;

    fun above_u64(extra: u128): aggregator_v2::Aggregator<u128> {
        let agg = aggregator_v2::create_unbounded_aggregator<u128>();
        assert!(aggregator_v2::try_add(&mut agg, TWO_POW_64 + extra), 100);
        agg
    }

    // Value 2^64 + 3 minus `amount`: returns whether it succeeded, then the value.
    public fun sub_u128(amount: u128): (bool, u128) {
        let agg = above_u64(3);
        let ok = aggregator_v2::try_sub(&mut agg, amount);
        (ok, aggregator_v2::read(&agg))
    }

    // Value 2^64 + 3 compared against `min_amount`.
    public fun at_least_u128(min_amount: u128): bool {
        let agg = above_u64(3);
        aggregator_v2::is_at_least(&agg, min_amount)
    }

    // A snapshot of 2^64 + 3 keeps its value after the aggregator changes.
    public fun snapshot_u128(): (u128, u128) {
        let agg = above_u64(3);
        let snap = aggregator_v2::snapshot(&agg);
        assert!(aggregator_v2::try_add(&mut agg, 1), 101);
        (aggregator_v2::read_snapshot(&snap), aggregator_v2::read(&agg))
    }

    public fun add_twice_u64(first: u64, second: u64): (bool, bool, u64) {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        let ok_first = aggregator_v2::try_add(&mut agg, first);
        let ok_second = aggregator_v2::try_add(&mut agg, second);
        (ok_first, ok_second, aggregator_v2::read(&agg))
    }

    public fun add_twice_u128(first: u128, second: u128): (bool, bool, u128) {
        let agg = aggregator_v2::create_unbounded_aggregator<u128>();
        let ok_first = aggregator_v2::try_add(&mut agg, first);
        let ok_second = aggregator_v2::try_add(&mut agg, second);
        (ok_first, ok_second, aggregator_v2::read(&agg))
    }
}

// 2^64 + 3 - 5 = 2^64 - 2; a 64-bit view of the value (3) would underflow.
// RUN: execute 0x1::main::sub_u128 --args 5
// CHECK-V2: results: true, 18446744073709551614

// RUN: execute 0x1::main::sub_u128 --args 18446744073709551619
// CHECK-V2: results: true, 0

// RUN: execute 0x1::main::sub_u128 --args 18446744073709551620
// CHECK-V2: results: false, 18446744073709551619

// RUN: execute 0x1::main::at_least_u128 --args 18446744073709551619
// CHECK-V2: results: true

// RUN: execute 0x1::main::at_least_u128 --args 18446744073709551620
// CHECK-V2: results: false

// A 64-bit view of the value (3) would not be at least 4.
// RUN: execute 0x1::main::at_least_u128 --args 4
// CHECK-V2: results: true

// RUN: execute 0x1::main::snapshot_u128
// CHECK-V2: results: 18446744073709551619, 18446744073709551620

// u64::MAX + 1 overflows: rejected, value unchanged.
// RUN: execute 0x1::main::add_twice_u64 --args 18446744073709551615, 1
// CHECK-V2: results: true, false, 18446744073709551615

// RUN: execute 0x1::main::add_twice_u64 --args 18446744073709551614, 1
// CHECK-V2: results: true, true, 18446744073709551615

// RUN: execute 0x1::main::add_twice_u128 --args 340282366920938463463374607431768211455, 1
// CHECK-V2: results: true, false, 340282366920938463463374607431768211455

// RUN: execute 0x1::main::add_twice_u128 --args 340282366920938463463374607431768211454, 1
// CHECK-V2: results: true, true, 340282366920938463463374607431768211455

// RUN: execute 0x1::aggregator_v2::read_packed_u64 --args 5, 5
// CHECK-V2: results: 5

// RUN: execute 0x1::aggregator_v2::read_packed_u64 --args 6, 5
// CHECK-V2: error: native function invariant violation: Aggregator read returned value greater than max: 6 > 5

// RUN: execute 0x1::aggregator_v2::read_packed_u128 --args 18446744073709551616, 18446744073709551616
// CHECK-V2: results: 18446744073709551616

// RUN: execute 0x1::aggregator_v2::read_packed_u128 --args 18446744073709551617, 18446744073709551616
// CHECK-V2: error: native function invariant violation: Aggregator read returned value greater than max: 18446744073709551617 > 18446744073709551616
