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
/// If you need to capture the value, without revealing it, use snapshot function instead,
/// which has no parallelism impact.
///
/// From parallelism considerations, there are three different levels of effects:
/// * enable full parallelism (cannot create conflicts):
///     max_value, create_*, snapshot, derive_string_concat
/// * enable speculative parallelism (generally parallel via branch prediction)
///     try_add, add, try_sub, sub, is_at_least
/// * create read/write conflicts, as if you were using a regular field
///     read, read_snapshot, read_derived_string
module aptos_framework::aggregator_v2 {
    use std::error;
    use std::features;
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

    /// Arguments passed to concat exceed max limit of 1024 bytes (for prefix and suffix together).
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
    struct AggregatorSnapshot<IntElement> has store, drop {
        value: IntElement,
    }

    struct DerivedStringSnapshot has store, drop {
        value: String,
        padding: vector<u8>,
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

    public fun create_aggregator_with_value<IntElement: copy + drop>(start_value: IntElement, max_value: IntElement): Aggregator<IntElement> {
        let aggregator = create_aggregator(max_value);
        add(&mut aggregator, start_value);
        aggregator
    }

    /// Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
    /// due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).
    ///
    /// Currently supported types for IntElement are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;

    public fun create_unbounded_aggregator_with_value<IntElement: copy + drop>(start_value: IntElement): Aggregator<IntElement> {
        let aggregator = create_unbounded_aggregator();
        add(&mut aggregator, start_value);
        aggregator
    }

    /// Adds `value` to aggregator.
    /// If addition would exceed the max_value, `false` is returned, and aggregator value is left unchanged.
    ///
    /// Parallelism info: This operation enables speculative parallelism.
    public native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    /// Adds `value` to aggregator, unconditionally.
    /// If addition would exceed the max_value, EAGGREGATOR_OVERFLOW exception will be thrown.
    ///
    /// Parallelism info: This operation enables speculative parallelism.
    public fun add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    ///
    /// Parallelism info: This operation enables speculative parallelism.
    public native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;

    // Subtracts `value` to aggregator, unconditionally.
    // If subtraction would result in a negative value, EAGGREGATOR_UNDERFLOW exception will be thrown.
    ///
    /// Parallelism info: This operation enables speculative parallelism.
    public fun sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    native fun is_at_least_impl<IntElement>(aggregator: &Aggregator<IntElement>, min_amount: IntElement): bool;

    /// Returns true if aggregator value is larger than or equal to the given `min_amount`, false otherwise.
    ///
    /// This operation is more efficient and much more parallelization friendly than calling `read(agg) > min_amount`.
    /// Until traits are deployed, `is_at_most`/`is_equal` utility methods can be derived from this one (assuming +1 doesn't overflow):
    /// - for `is_at_most(agg, max_amount)`, you can do `!is_at_least(max_amount + 1)`
    /// - for `is_equal(agg, value)`, you can do `is_at_least(value) && !is_at_least(value + 1)`
    ///
    /// Parallelism info: This operation enables speculative parallelism.
    public fun is_at_least<IntElement>(aggregator: &Aggregator<IntElement>, min_amount: IntElement): bool {
        assert!(features::aggregator_v2_is_at_least_api_enabled(), EAGGREGATOR_API_V2_NOT_ENABLED);
        is_at_least_impl(aggregator, min_amount)
    }

    // TODO waiting for integer traits
    // public fun is_at_most<IntElement>(aggregator: &Aggregator<IntElement>, max_amount: IntElement): bool {
    //     !is_at_least(max_amount + 1)
    // }

    // TODO waiting for integer traits
    // public fun is_equal<IntElement>(aggregator: &Aggregator<IntElement>, value: IntElement): bool {
    //     is_at_least(value) && !is_at_least(value + 1)
    // }

    /// Returns a value stored in this aggregator.
    /// Note: This operation is resource-intensive, and reduces parallelism.
    /// If you need to capture the value, without revealing it, use snapshot function instead,
    /// which has no parallelism impact.
    /// If called in a transaction that also modifies the aggregator, or has other read/write conflicts,
    /// it will sequentialize that transaction. (i.e. up to concurrency_level times slower)
    /// If called in a separate transaction (i.e. after transaction that modifies aggregator), it might be
    /// up to two times slower.
    ///
    /// Parallelism info: This operation *prevents* speculative parallelism.
    public native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;

    /// Returns a wrapper of a current value of an aggregator
    /// Unlike read(), it is fast and avoids sequential dependencies.
    ///
    /// Parallelism info: This operation enables parallelism.
    public native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;

    /// Creates a snapshot of a given value.
    /// Useful for when object is sometimes created via snapshot() or string_concat(), and sometimes directly.
    public native fun create_snapshot<IntElement: copy + drop>(value: IntElement): AggregatorSnapshot<IntElement>;

    /// Returns a value stored in this snapshot.
    /// Note: This operation is resource-intensive, and reduces parallelism.
    /// (Especially if called in a transaction that also modifies the aggregator,
    /// or has other read/write conflicts)
    ///
    /// Parallelism info: This operation *prevents* speculative parallelism.
    public native fun read_snapshot<IntElement>(snapshot: &AggregatorSnapshot<IntElement>): IntElement;

    /// Returns a value stored in this DerivedStringSnapshot.
    /// Note: This operation is resource-intensive, and reduces parallelism.
    /// (Especially if called in a transaction that also modifies the aggregator,
    /// or has other read/write conflicts)
    ///
    /// Parallelism info: This operation *prevents* speculative parallelism.
    public native fun read_derived_string(snapshot: &DerivedStringSnapshot): String;

    /// Creates a DerivedStringSnapshot of a given value.
    /// Useful for when object is sometimes created via string_concat(), and sometimes directly.
    public native fun create_derived_string(value: String): DerivedStringSnapshot;

    /// Concatenates `before`, `snapshot` and `after` into a single string.
    /// snapshot passed needs to have integer type - currently supported types are u64 and u128.
    /// Raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.
    /// If length of prefix and suffix together exceeds 1024 bytes, ECONCAT_STRING_LENGTH_TOO_LARGE is raised.
    ///
    /// Parallelism info: This operation enables parallelism.
    public native fun derive_string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): DerivedStringSnapshot;

    // ===== DEPRECATE/NOT YET IMPLEMENTED ====

    #[deprecated]
    /// NOT YET IMPLEMENTED, always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.
    public native fun copy_snapshot<IntElement: copy + drop>(snapshot: &AggregatorSnapshot<IntElement>): AggregatorSnapshot<IntElement>;

    #[deprecated]
    /// DEPRECATED, use derive_string_concat() instead. always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.
    public native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;

    #[verify_only]
    fun verify_aggregator_try_add_sub(): Aggregator<u64> {
        let agg = create_aggregator(10);
        spec {
            assert spec_get_max_value(agg) == 10;
            assert spec_get_value(agg) == 0;
        };
        let x = try_add(&mut agg, 5);
        spec {
            assert x;
            assert is_at_least(agg, 5);
        };
        let y = try_sub(&mut agg, 6);
        spec {
            assert !y;
            assert spec_get_value(agg) == 5;
            assert spec_get_max_value(agg) == 10;
        };
        let y = try_sub(&mut agg, 4);
        spec {
            assert y;
            assert spec_get_value(agg) == 1;
            assert spec_get_max_value(agg) == 10;
        };
        let x = try_add(&mut agg, 11);
        spec {
            assert !x;
            assert spec_get_value(agg) == 1;
            assert spec_get_max_value(agg) == 10;
        };
        let x = try_add(&mut agg, 9);
        spec {
            assert x;
            assert spec_get_value(agg) == 10;
            assert spec_get_max_value(agg) == 10;
        };
        agg
    }

    spec verify_aggregator_try_add_sub{
        ensures spec_get_max_value(result) == 10;
        ensures spec_get_value(result) == 10;
        ensures read(result) == 10;
    }

    #[verify_only]
    fun verify_aggregator_add_sub(sub_value: u64, add_value: u64) {
        let agg = create_aggregator(10);
        add(&mut agg, add_value);
        spec {
            assert spec_get_value(agg) == add_value;
        };
        sub(&mut agg, sub_value);
        spec {
            assert spec_get_value(agg) == add_value - sub_value;
        };
    }

    spec verify_aggregator_add_sub(sub_value: u64, add_value: u64) {
        pragma aborts_if_is_strict;
        aborts_if add_value > 10;
        aborts_if sub_value > add_value;
    }

    #[verify_only]
    fun verify_correct_read() {
        let snapshot = create_snapshot(42);
        spec {
            assert spec_read_snapshot(snapshot) == 42;
        };
        let derived = create_derived_string(std::string::utf8(b"42"));
        spec {
            assert spec_read_derived_string(derived).bytes == b"42";
        };
    }

    #[verify_only]
    fun verify_invalid_read(aggregator: &Aggregator<u8>): u8 {
        read(aggregator)
    }
    spec verify_invalid_read {
        aborts_if true;
    }

    #[verify_only]
    fun verify_invalid_is_least(aggregator: &Aggregator<u8>): bool {
        is_at_least(aggregator, 0)
    }
    spec verify_invalid_is_least {
        aborts_if true;
    }

    #[verify_only]
    fun verify_copy_not_yet_supported() {
        let snapshot = create_snapshot(42);
        copy_snapshot(&snapshot);
    }

    spec verify_copy_not_yet_supported {
        aborts_if true;
    }

    #[verify_only]
    fun verify_string_concat1() {
        let snapshot = create_snapshot(42);
        let derived = derive_string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        spec {
            assert spec_read_derived_string(derived).bytes ==
                concat(b"before", concat(spec_get_string_value(snapshot).bytes, b"after"));
        };
    }

    #[verify_only]
    fun verify_aggregator_generic<IntElement1: copy + drop, IntElement2: copy+drop>(): (Aggregator<IntElement1>,  Aggregator<IntElement2>){
        let x = create_unbounded_aggregator<IntElement1>();
        let y = create_unbounded_aggregator<IntElement2>();
        (x, y)
    }
    spec verify_aggregator_generic <IntElement1: copy + drop, IntElement2: copy+drop>(): (Aggregator<IntElement1>,  Aggregator<IntElement2>) {
        use aptos_std::type_info;
        aborts_if type_info::type_name<IntElement1>().bytes != b"u64" && type_info::type_name<IntElement1>().bytes != b"u128";
        aborts_if type_info::type_name<IntElement2>().bytes != b"u64" && type_info::type_name<IntElement2>().bytes != b"u128";
    }

    #[verify_only]
    fun verify_aggregator_generic_add<IntElement: copy + drop>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        try_add(aggregator, value);
        is_at_least_impl(aggregator, value);
        // cannot specify aborts_if condition for generic `add`
        // because comparison is not supported by IntElement
        add(aggregator, value);
    }
    spec verify_aggregator_generic_add<IntElement: copy + drop>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        use aptos_std::type_info;
        aborts_if type_info::type_name<IntElement>().bytes != b"u64" && type_info::type_name<IntElement>().bytes != b"u128";
    }

    #[verify_only]
    fun verify_aggregator_generic_sub<IntElement: copy + drop>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        try_sub(aggregator, value);
        // cannot specify aborts_if condition for generic `sub`
        // because comparison is not supported by IntElement
        sub(aggregator, value);
    }
    spec verify_aggregator_generic_sub<IntElement: copy + drop>(aggregator: &mut Aggregator<IntElement>, value: IntElement) {
        use aptos_std::type_info;
        aborts_if type_info::type_name<IntElement>().bytes != b"u64" && type_info::type_name<IntElement>().bytes != b"u128";
    }

    #[verify_only]
    fun verify_aggregator_invalid_type1() {
        create_unbounded_aggregator<u8>();
    }
    spec verify_aggregator_invalid_type1 {
        aborts_if true;
    }

    #[verify_only]
    fun verify_snapshot_invalid_type1() {
        use std::option;
        create_snapshot(option::some(42));
    }
    spec verify_snapshot_invalid_type1 {
        aborts_if true;
    }

    #[verify_only]
    fun verify_snapshot_invalid_type2() {
        create_snapshot(vector[42]);
    }

    spec verify_snapshot_invalid_type2 {
        aborts_if true;
    }

    #[verify_only]
    fun verify_aggregator_valid_type() {
        let _agg_1 = create_unbounded_aggregator<u64>();
        spec {
            assert spec_get_max_value(_agg_1) == MAX_U64;
        };
        let _agg_2 = create_unbounded_aggregator<u128>();
        spec {
            assert spec_get_max_value(_agg_2) == MAX_U128;
        };
        create_aggregator<u64>(5);
        create_aggregator<u128>(5);
    }

    spec verify_aggregator_valid_type {
        aborts_if false;
    }

    // ========================================

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

        let derived = create_derived_string(std::string::utf8(b"42"));
        assert!(read_derived_string(&derived) == std::string::utf8(b"42"), 0);
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
        let derived = derive_string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(read_derived_string(&derived) == std::string::utf8(b"before42after"), 0);
    }

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
