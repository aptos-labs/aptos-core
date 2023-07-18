/// This module provides an interface for aggregators (version 2).
module aptos_framework::aggregator_v2 {
    use std::error;
    use std::string::String;

    /// The value of aggregator overflows. Raised by uncoditional add() call
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// Element type of the aggregator not supported. Raised by create_aggregator() call,
    /// if Element type is not supported (currently u64 or u128).
    const EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED: u64 = 3;

    /// The generic type supplied to the aggregator snapshot is not supported.
    const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 5;

    /// The aggregator snapshots feature flag is not enabled.
    const EAGGREGATOR_SNAPSHOTS_NOT_ENABLED: u64 = 6;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    ///
    /// Currently supported types for Element are u64 and u128.
    struct Aggregator<Element> has store {
        value: Element,
        max_value: Element,
    }

    struct AggregatorSnapshot<Element> has store, drop {
        value: Element,
    }

    /// Returns `max_value` exceeding which aggregator overflows.
    public fun max_value<Element: copy + drop>(aggregator: &Aggregator<Element>): Element {
        aggregator.max_value
    }

    /// Creates new aggregator, with given 'max_value'.
    ///
    /// Currently supported types for Element are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_aggregator<Element: copy + drop>(max_value: Element): Aggregator<Element>;

    /// Adds `value` to aggregator.
    /// If addition would exceed the max_value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_add<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the max_value, EAGGREGATOR_OVERFLOW exception will be thrown
    public fun add<Element>(aggregator: &mut Aggregator<Element>, value: Element) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_sub<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If subtraction would result in a negative value, EAGGREGATOR_UNDERFLOW exception will be thrown
    public fun sub<Element>(aggregator: &mut Aggregator<Element>, value: Element) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    /// Returns a value stored in this aggregator.
    public native fun read<Element>(aggregator: &Aggregator<Element>): Element;

    public native fun snapshot<Element>(aggregator: &Aggregator<Element>): AggregatorSnapshot<Element>;

    public native fun create_snapshot<Element: copy + drop>(value: Element): AggregatorSnapshot<Element>;

    public native fun copy_snapshot<Element: copy + drop>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;

    public native fun string_concat<Element>(before: String, snapshot: &AggregatorSnapshot<Element>, after: String): AggregatorSnapshot<String>;

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
