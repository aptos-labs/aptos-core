/// This module provides an interface for aggregators (version 2). Aggregators are
/// similar to unsigned integers and support addition and subtraction (aborting on
/// underflow or on overflowing a custom upper limit). The difference from integers
/// is that aggregators allow to perform both additions and subtractions in parallel
/// across multiple transactions, enabling parallel execution. For example, if the
/// first transaction is doing `try_add(X, 1)` for aggregator `X`, and the second is
/// doing `try_sub(X,3)`, they can be executed in parallel avoiding a read-modify-write
/// dependency.
/// However, reading the aggregator value (i.e. calling `read(X)`) is a resource-intensive
/// operation that also reduced parallelism, and should be avoided as much as possible.
module aptos_framework::aggregator_v2 {
    use std::error;
    use std::string::String;

    /// The value of aggregator overflows. Raised by uncoditional add() call
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// The generic type supplied to the aggregator snapshot is not supported.
    const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 5;

    /// The aggregator api v2 feature flag is not enabled.
    const EAGGREGATOR_API_V2_NOT_ENABLED: u64 = 6;

    /// The generic type supplied to the aggregator is not supported.
    const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 7;

    /// Arguments passed to concat exceed max limit of 256 bytes (for prefix and suffix together).
    const ECONCAT_STRING_LENGTH_TOO_LARGE: u64 = 8;

    /// The native aggregator function, that is in the move file, is not yet supported.
    /// and any calls will raise this error.
    const EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED: u64 = 9;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    ///
    /// Currently supported types for IntElement are u64 and u128.
    struct Aggregator<IntElement> has store, drop {
        value: IntElement,
        max_value: IntElement,
    }

    /// Represents a constant value, that was derived from an aggregator at given instant in time.
    /// Unlike read() and storing the value directly, this enables parallel execution of transactions,
    /// while storing snapshot of aggregator state elsewhere.
    struct AggregatorSnapshot<Element> has store, drop {
        value: Element,
    }

    /// Returns `max_value` exceeding which aggregator overflows.
    public fun max_value<IntElement: copy + drop>(aggregator: &Aggregator<IntElement>): IntElement {
        aggregator.max_value
    }

    /// Creates new aggregator, with given 'max_value'.
    ///
    /// Currently supported types for IntElement are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_aggregator<IntElement: copy + drop>(max_value: IntElement): Aggregator<IntElement>;

    /// Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
    /// due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).
    ///
    /// Currently supported types for IntElement are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;

    /// Adds `value` to aggregator.
    /// If addition would exceed the max_value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the max_value, EAGGREGATOR_OVERFLOW exception will be thrown.
    public fun add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    // Subtracts `value` to aggregator, uncoditionally.
    // If subtraction would result in a negative value, EAGGREGATOR_UNDERFLOW exception will be thrown.
    public fun sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    /// Returns a value stored in this aggregator.
    /// Note: This operation is resource-intensive, and reduces parallelism.
    /// (Especially if called in a transaction that also modifies the aggregator,
    /// or has other read/write conflicts)
    public native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;

    /// Returns a wrapper of a current value of an aggregator
    /// Unlike read(), it is fast and avoids sequential dependencies.
    public native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;

    /// Creates a snapshot of a given value.
    /// Useful for when object is sometimes created via snapshot() or string_concat(), and sometimes directly.
    public native fun create_snapshot<Element: copy + drop>(value: Element): AggregatorSnapshot<Element>;

    /// NOT YET IMPLEMENTED, always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.
    public native fun copy_snapshot<Element: copy + drop>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;

    /// Returns a value stored in this snapshot.
    /// Note: This operation is resource-intensive, and reduces parallelism.
    /// (Especially if called in a transaction that also modifies the aggregator,
    /// or has other read/write conflicts)
    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;

    /// Concatenates `before`, `snapshot` and `after` into a single string.
    /// snapshot passed needs to have integer type - currently supported types are u64 and u128.
    /// Raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.
    /// If length of prefix and suffix together exceed 256 bytes, ECONCAT_STRING_LENGTH_TOO_LARGE is raised.
    public native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;

    #[test]
    fun test_aggregator() {
        let agg = create_aggregator(10);
        assert!(try_add(&mut agg, 5), 1);
        assert!(try_add(&mut agg, 5), 2);
        assert!(read(&agg) == 10, 3);
        assert!(!try_add(&mut agg, 5), 4);
        assert!(read(&agg) == 10, 5);
        assert!(try_sub(&mut agg, 5), 6);
        assert!(read(&agg) == 5, 7);

        let snap = snapshot(&agg);
        assert!(try_add(&mut agg, 2), 8);
        assert!(read(&agg) == 7, 9);
        assert!(read_snapshot(&snap) == 5, 10);
    }

    #[test]
    fun test_correct_read() {
        let snapshot = create_snapshot(42);
        assert!(read_snapshot(&snapshot) == 42, 0);

        let snapshot = create_snapshot(std::string::utf8(b"42"));
        assert!(read_snapshot(&snapshot) == std::string::utf8(b"42"), 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x030009, location = Self)]
    fun test_copy_not_yet_supported() {
        let snapshot = create_snapshot(42);
        copy_snapshot(&snapshot);
    }

    #[test]
    fun test_string_concat1() {
        let snapshot = create_snapshot(42);
        let snapshot2 = string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x030005, location = Self)]
    fun test_string_concat_from_string_not_supported() {
        let snapshot = create_snapshot<String>(std::string::utf8(b"42"));
        string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
    }

    // Tests commented out, as flag used in rust cannot be disabled.

    // #[test(fx = @std)]
    // #[expected_failure(abort_code = 0x030006, location = Self)]
    // fun test_snapshot_feature_not_enabled(fx: &signer) {
    //     use std::features;
    //     use aptos_framework::reconfiguration;
    //     let feature = features::get_aggregator_v2_api_feature();
    //     features::change_feature_flags(fx, vector[], vector[feature]);
    //     reconfiguration::reconfigure_for_test();
    //     create_snapshot(42);
    // }

    // #[test(fx = @std)]
    // #[expected_failure(abort_code = 0x030006, location = Self)]
    // fun test_aggregator_feature_not_enabled(fx: &signer) {
    //     use std::features;
    //     use aptos_framework::reconfiguration;
    //     let feature = features::get_aggregator_v2_api_feature();
    //     features::change_feature_flags(fx, vector[], vector[feature]);
    //     reconfiguration::reconfigure_for_test();
    //     create_aggregator(42);
    // }

    #[test]
    #[expected_failure(abort_code = 0x030007, location = Self)]
    fun test_aggregator_invalid_type1() {
        create_unbounded_aggregator<u8>();
    }

    #[test]
    fun test_aggregator_valid_type() {
        create_unbounded_aggregator<u64>();
        create_unbounded_aggregator<u128>();
        create_aggregator<u64>(5);
        create_aggregator<u128>(5);
    }

    #[test]
    #[expected_failure(abort_code = 0x030005, location = Self)]
    fun test_snpashot_invalid_type1() {
        use std::option;
        create_snapshot(option::some(42));
    }

    #[test]
    #[expected_failure(abort_code = 0x030005, location = Self)]
    fun test_snpashot_invalid_type2() {
        create_snapshot(vector[42]);
    }
}
