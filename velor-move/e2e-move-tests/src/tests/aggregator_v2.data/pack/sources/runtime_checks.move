/// Aggregators and other structs that use delayed fields have certain restrictions
/// imposed by runtime, e.g. they cannot be compared, serialized, etc. because delayed
/// field values get exchanged with unique identifiers at runtime.
module 0x1::runtime_checks {
    use std::bcs;
    use std::string;
    use velor_std::string_utils;
    use velor_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot, DerivedStringSnapshot};

    //
    // Structs and constructors.
    //

    struct StructWithAggregator<IntTy: copy + drop> has drop {
        aggregator: Aggregator<IntTy>,
    }

    struct StructWithAggregatorSnapshot<IntTy: copy + drop> has drop {
        snapshot: AggregatorSnapshot<IntTy>,
    }

    struct StructWithDerivedStringSnapshot has drop {
        derived_string_snapshot: DerivedStringSnapshot,
    }

    fun with_aggregator<IntTy: copy + drop>(): StructWithAggregator<IntTy> {
        StructWithAggregator {
            aggregator: aggregator_v2::create_unbounded_aggregator<IntTy>()
        }
    }

    fun with_snapshot<IntTy: copy + drop>(value: IntTy): StructWithAggregatorSnapshot<IntTy> {
        StructWithAggregatorSnapshot {
            snapshot: aggregator_v2::create_snapshot(value)
        }
    }

    fun with_derived_string_snapshot(str: vector<u8>): StructWithDerivedStringSnapshot {
        StructWithDerivedStringSnapshot {
            derived_string_snapshot: aggregator_v2::create_derived_string(string::utf8(str))
        }
    }

    // TODO[agg_v2](cleanup): Move to aggregator Move file tests when feature flag is enabled.

    //
    // Equality.
    //

    public entry fun test_equality_with_aggregators_I() {
        let a = with_aggregator<u64>();
        let b = with_aggregator<u64>();
        aggregator_v2::try_add(&mut b.aggregator, 10);
        let _ = a != b;
    }

    public entry fun test_equality_with_aggregators_II() {
        let a = with_aggregator<u64>();
        let b = with_aggregator<u64>();
        let _ = a == b;
    }

    public entry fun test_equality_with_aggregators_III() {
        let a = with_aggregator<u64>();
        let _ = &a == &a;
    }

    public entry fun test_equality_with_snapshots_I() {
        let a = with_snapshot<u64>(0);
        let b = with_snapshot<u64>(10);
        let _ = a != b;
    }

    public entry fun test_equality_with_snapshots_II() {
        let a = with_snapshot<u64>(0);
        let b = with_snapshot<u64>(0);
        let _ = a == b;
    }

    public entry fun test_equality_with_snapshots_III() {
        let a = with_snapshot<u64>(0);
        let _ = &a == &a;
    }

    public entry fun test_equality_with_derived_string_snapshots_I() {
        let a = with_derived_string_snapshot(b"aaa");
        let b = with_derived_string_snapshot(b"bbb");
        let _ = a != b;
    }

    public entry fun test_equality_with_derived_string_snapshots_II() {
        let a = with_derived_string_snapshot(b"aaa");
        let b = with_derived_string_snapshot(b"aaa");
        let _ = a == b;
    }

    public entry fun test_equality_with_derived_string_snapshots_III() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = &a == &a;
    }

    //
    // Serialization.
    //

    public entry fun test_serialization_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = bcs::to_bytes(&a);
    }

    public entry fun test_serialization_with_snapshots() {
        let a = with_snapshot<u64>(0);
        let _ = bcs::to_bytes(&a);
    }

    public entry fun test_serialization_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = bcs::to_bytes(&a);
    }

    public entry fun test_serialized_size_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = bcs::serialized_size(&a);
    }

    public entry fun test_serialized_size_with_snapshots() {
        let a = with_snapshot<u128>(0);
        let _ = bcs::serialized_size(&a);
    }

    public entry fun test_serialized_size_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = bcs::serialized_size(&a);
    }

    //
    // String utils:
    //   - to_string
    //   - to_string_with_canonical_addresses
    //   - to_string_with_integer_types
    //   - debug_string
    //
    // TODO[agg_v2]: consider formats of lists?

    public entry fun test_to_string_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = string_utils::to_string(&a);
    }

    public entry fun test_to_string_with_snapshots() {
        let a = with_snapshot<u64>(0);
        let _ = string_utils::to_string(&a);
    }

    public entry fun test_to_string_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = string_utils::to_string(&a);
    }

    public entry fun test_to_string_with_canonical_addresses_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = string_utils::to_string_with_canonical_addresses(&a);
    }

    public entry fun test_to_string_with_canonical_addresses_with_snapshots() {
        let a = with_snapshot<u64>(0);
        let _ = string_utils::to_string_with_canonical_addresses(&a);
    }

    public entry fun test_to_string_with_canonical_addresses_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = string_utils::to_string_with_canonical_addresses(&a);
    }

    public entry fun test_to_string_with_integer_types_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = string_utils::to_string_with_integer_types(&a);
    }

    public entry fun test_to_string_with_integer_types_with_snapshots() {
        let a = with_snapshot<u64>(0);
        let _ = string_utils::to_string_with_integer_types(&a);
    }

    public entry fun test_to_string_with_integer_types_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = string_utils::to_string_with_integer_types(&a);
    }

    public entry fun test_debug_string_with_aggregators() {
        let a = with_aggregator<u64>();
        let _ = string_utils::debug_string(&a);
    }

    public entry fun test_debug_string_with_snapshots() {
        let a = with_snapshot<u64>(0);
        let _ = string_utils::debug_string(&a);
    }

    public entry fun test_debug_string_with_derived_string_snapshots() {
        let a = with_derived_string_snapshot(b"aaa");
        let _ = string_utils::debug_string(&a);
    }
}
