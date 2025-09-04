/// This module provides an interface to aggregate integers either via
/// aggregator (parallelizable) or via normal integers.
module velor_framework::optional_aggregator {
    use std::error;
    use std::option::{Self, Option};

    use velor_framework::aggregator_factory;
    use velor_framework::aggregator::{Self, Aggregator};

    friend velor_framework::coin;
    friend velor_framework::fungible_asset;

    /// The value of aggregator underflows (goes below zero). Raised by native code.
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// Aggregator feature is not supported. Raised by native code.
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// OptionalAggregator (Agg V1) switch not supported any more.
    const ESWITCH_DEPRECATED: u64 = 3;

    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// Wrapper around integer with a custom overflow limit. Supports add, subtract and read just like `Aggregator`.
    struct Integer has store {
        value: u128,
        limit: u128,
    }

    /// Creates a new integer which overflows on exceeding a `limit`.
    fun new_integer(limit: u128): Integer {
        Integer {
            value: 0,
            limit,
        }
    }

    /// Adds `value` to integer. Aborts on overflowing the limit.
    fun add_integer(integer: &mut Integer, value: u128) {
        assert!(
            value <= (integer.limit - integer.value),
            error::out_of_range(EAGGREGATOR_OVERFLOW)
        );
        integer.value = integer.value + value;
    }

    /// Subtracts `value` from integer. Aborts on going below zero.
    fun sub_integer(integer: &mut Integer, value: u128) {
        assert!(value <= integer.value, error::out_of_range(EAGGREGATOR_UNDERFLOW));
        integer.value = integer.value - value;
    }

    /// Returns an overflow limit of integer.
    fun limit(integer: &Integer): u128 {
        integer.limit
    }

    /// Returns a value stored in this integer.
    fun read_integer(integer: &Integer): u128 {
        integer.value
    }

    /// Destroys an integer.
    fun destroy_integer(integer: Integer) {
        let Integer { value: _, limit: _ } = integer;
    }

    /// Contains either an aggregator or a normal integer, both overflowing on limit.
    struct OptionalAggregator has store {
        // Parallelizable.
        aggregator: Option<Aggregator>,
        // Non-parallelizable.
        integer: Option<Integer>,
    }

    /// Creates a new optional aggregator.
    public(friend) fun new(parallelizable: bool): OptionalAggregator {
        if (parallelizable) {
            OptionalAggregator {
                aggregator: option::some(aggregator_factory::create_aggregator_internal()),
                integer: option::none(),
            }
        } else {
            OptionalAggregator {
                aggregator: option::none(),
                integer: option::some(new_integer(MAX_U128)),
            }
        }
    }

    /// Switches between parallelizable and non-parallelizable implementations.
    public fun switch(_optional_aggregator: &mut OptionalAggregator) {
        abort error::invalid_state(ESWITCH_DEPRECATED)
    }

    /// Destroys optional aggregator.
    public fun destroy(optional_aggregator: OptionalAggregator) {
        if (is_parallelizable(&optional_aggregator)) {
            destroy_optional_aggregator(optional_aggregator);
        } else {
            destroy_optional_integer(optional_aggregator);
        }
    }

    /// Destroys parallelizable optional aggregator and returns its limit.
    fun destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 {
        let OptionalAggregator { aggregator, integer } = optional_aggregator;
        let limit = aggregator::limit(option::borrow(&aggregator));
        aggregator::destroy(option::destroy_some(aggregator));
        option::destroy_none(integer);
        limit
    }

    /// Destroys non-parallelizable optional aggregator and returns its limit.
    fun destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 {
        let OptionalAggregator { aggregator, integer } = optional_aggregator;
        let limit = limit(option::borrow(&integer));
        destroy_integer(option::destroy_some(integer));
        option::destroy_none(aggregator);
        limit
    }

    /// Adds `value` to optional aggregator, aborting on exceeding the `limit`.
    public fun add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow_mut(&mut optional_aggregator.aggregator);
            aggregator::add(aggregator, value);
        } else {
            let integer = option::borrow_mut(&mut optional_aggregator.integer);
            add_integer(integer, value);
        }
    }

    /// Subtracts `value` from optional aggregator, aborting on going below zero.
    public fun sub(optional_aggregator: &mut OptionalAggregator, value: u128) {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow_mut(&mut optional_aggregator.aggregator);
            aggregator::sub(aggregator, value);
        } else {
            let integer = option::borrow_mut(&mut optional_aggregator.integer);
            sub_integer(integer, value);
        }
    }

    /// Returns the value stored in optional aggregator.
    public fun read(optional_aggregator: &OptionalAggregator): u128 {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow(&optional_aggregator.aggregator);
            aggregator::read(aggregator)
        } else {
            let integer = option::borrow(&optional_aggregator.integer);
            read_integer(integer)
        }
    }

    /// Returns true if optional aggregator uses parallelizable implementation.
    public fun is_parallelizable(optional_aggregator: &OptionalAggregator): bool {
        option::is_some(&optional_aggregator.aggregator)
    }

    #[test(account = @velor_framework)]
    #[expected_failure(abort_code = 0x030003, location = Self)]
    fun optional_aggregator_swith_fail_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(true);
        switch(&mut aggregator);
        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    fun optional_aggregator_test_integer(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);

        let aggregator = new(false);
        assert!(!is_parallelizable(&aggregator), 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 5, 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 20, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 10, 0);

        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    fun optional_aggregator_test_aggregator(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(true);
        assert!(is_parallelizable(&aggregator), 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 5, 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 20, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 10, 0);

        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    fun optional_aggregator_destroy_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);

        let aggregator = new(false);
        destroy(aggregator);

        let aggregator = new(true);
        destroy(aggregator);

        let aggregator = new(false);
        assert!(destroy_optional_integer(aggregator) == MAX_U128, 0);

        let aggregator = new(true);
        assert!(destroy_optional_aggregator(aggregator) == MAX_U128, 0);
    }

    #[test(account = @velor_framework)]
    #[expected_failure(abort_code = 0x020001, location = Self)]
    fun non_parallelizable_aggregator_overflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(false);
        add(&mut aggregator, MAX_U128 - 15);

        // Overflow!
        add(&mut aggregator, 16);

        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    #[expected_failure(abort_code = 0x020002, location = Self)]
    fun non_parallelizable_aggregator_underflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(false);

        // Underflow!
        sub(&mut aggregator, 100);
        add(&mut aggregator, 100);

        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    #[expected_failure(abort_code = 0x020001, location = velor_framework::aggregator)]
    fun parallelizable_aggregator_overflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(true);
        add(&mut aggregator, MAX_U128 - 15);

        // Overflow!
        add(&mut aggregator, 16);

        destroy(aggregator);
    }

    #[test(account = @velor_framework)]
    #[expected_failure(abort_code = 0x020002, location = velor_framework::aggregator)]
    fun parallelizable_aggregator_underflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(true);

        // Underflow!
        add(&mut aggregator, 99);
        sub(&mut aggregator, 100);
        add(&mut aggregator, 100);

        destroy(aggregator);
    }
}
