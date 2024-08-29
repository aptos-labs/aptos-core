/// This module provides an interface to aggregate integers either via
/// aggregator (parallelizable) or via normal integers.
module aptos_framework::optional_aggregator {
    use std::error;
    use std::option::{Self, Option};

    use aptos_framework::aggregator::{Self, Aggregator};

    friend aptos_framework::coin;
    friend aptos_framework::fungible_asset;

    /// The value of aggregator underflows (goes below zero). Raised by native code.
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// Aggregator feature is not supported. Raised by native code.
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// OptionalAggregator (Agg V1) switch not supported any more
    const ESWITCH_DEPRECATED: u64 = 3;

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

    public(friend) fun new_empty(): OptionalAggregator {
        OptionalAggregator {
            aggregator: option::none(),
            integer: option::none(),
        }
    }

    public(friend) fun is_empty(optional_aggregator: &OptionalAggregator): bool {
        !option::is_some(&optional_aggregator.aggregator) && !option::is_some(&optional_aggregator.integer)
    }

    public(friend) fun make_empty(optional_aggregator: &mut OptionalAggregator): u128 {
        if (is_parallelizable(optional_aggregator)) {
            let aggregator = option::extract(&mut optional_aggregator.aggregator);
            let value = aggregator::read(&aggregator);
            aggregator::destroy(aggregator);
            value
        } else {
            let integer = option::extract(&mut optional_aggregator.integer);
            let value = read_integer(&integer);
            destroy_integer(integer);
            value
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
}
