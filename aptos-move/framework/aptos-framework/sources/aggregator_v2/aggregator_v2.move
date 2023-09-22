/// This module provides an interface for aggregators (version 2).
module aptos_framework::aggregator_v2 {
    use std::error;
    use std::string::String;
    use aptos_std::type_info;

    /// The value of aggregator overflows. Raised by uncoditional add() call
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// The generic type supplied to the aggregator snapshot is not supported.
    const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 5;

    /// The aggregator snapshots feature flag is not enabled.
    const EAGGREGATOR_SNAPSHOTS_NOT_ENABLED: u64 = 6;

    /// The generic type supplied to the aggregator is not supported.
    const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 7;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    ///
    /// Currently supported types for Element are u64 and u128.
    struct Aggregator<IntElement> has store {
        value: IntElement,
        max_value: IntElement,
    }

    struct AggregatorSnapshot<Element> has store, drop {
        value: Element,
    }

    /// Returns `max_value` exceeding which aggregator overflows.
    public fun max_value<IntElement: copy + drop>(aggregator: &Aggregator<IntElement>): IntElement {
        aggregator.max_value
    }

    /// Creates new aggregator, with given 'max_value'.
    ///
    /// Currently supported types for Element are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_aggregator<IntElement: copy + drop>(max_value: IntElement): Aggregator<IntElement>;

    /// Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
    /// due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).
    ///
    /// Currently supported types for Element are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;

    /// Adds `value` to aggregator.
    /// If addition would exceed the max_value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the max_value, EAGGREGATOR_OVERFLOW exception will be thrown
    public fun add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If subtraction would result in a negative value, EAGGREGATOR_UNDERFLOW exception will be thrown
    public fun sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    /// Returns a value stored in this aggregator.
    public native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;

    public native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;

    public native fun create_snapshot<Element: copy + drop>(value: Element): AggregatorSnapshot<Element>;

    public native fun copy_snapshot<Element: copy + drop>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;

    /// Concatenates `before`, `snapshot` and `after` into a single string.
    /// snapshot passed needs to have integer type - currently supported types are u64 and u128.
    /// raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.
    public native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;

    // #[test(fx = @std)]
    // public fun test_correct_read(fx: &signer) {
    //     use std::features;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     let snapshot = create_snapshot(42);
    //     let snapshot2 = copy_snapshot(&snapshot);
    //     assert!(read_snapshot(&snapshot) == 42, 0);
    //     assert!(read_snapshot(&snapshot2) == 42, 0);
    // }

    // #[test(fx = @std)]
    // public fun test_correct_read_string(fx: &signer) {
    //     use std::features;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     let snapshot = create_snapshot(std::string::utf8(b"42"));
    //     let snapshot2 = copy_snapshot(&snapshot);
    //     assert!(read_snapshot(&snapshot) == std::string::utf8(b"42"), 0);
    //     assert!(read_snapshot(&snapshot2) == std::string::utf8(b"42"), 0);
    // }

    // #[test(fx = @std)]
    // public fun test_string_concat1(fx: &signer) {
    //     use std::features;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     let snapshot = create_snapshot(42);
    //     let snapshot2 = string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
    //     assert!(read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    // }

    // #[test(fx = @std)]
    // public fun test_string_concat2(fx: &signer) {
    //     use std::features;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     let snapshot = create_snapshot<String>(std::string::utf8(b"42"));
    //     let snapshot2 = string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
    //     assert!(read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    // }

    // #[test]
    // #[expected_failure(abort_code = 0x030006, location = Self)]
    // public fun test_snapshot_feature_not_enabled() {
    //     create_snapshot(42);
    // }

    // #[test(fx = @std)]
    // #[expected_failure(abort_code = 0x030005, location = Self)]
    // public fun test_snpashot_invalid_type1(fx: &signer) {
    //     use std::features;
    //     use std::option;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     create_snapshot(option::some(42));
    // }

    // #[test(fx = @std)]
    // #[expected_failure(abort_code = 0x030005, location = Self)]
    // public fun test_snpashot_invalid_type2(fx: &signer) {
    //     use std::features;
    //     let feature = features::get_aggregator_snapshots_feature();
    //     features::change_feature_flags(fx, vector[feature], vector[]);

    //     create_snapshot(vector[42]);
    // }
}
