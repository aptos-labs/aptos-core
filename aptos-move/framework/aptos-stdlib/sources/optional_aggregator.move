/// This module provides an interface to aggregate integers either via
/// aggregator (parallelizable) or via normal integers.
module aptos_std::optional_aggregator {
    use std::error;
    use std::option::{Self, Option};

    use aptos_std::aggregator_factory;
    use aptos_std::aggregator::{Self, Aggregator};

    // These error codes are produced by `Aggregator` and used by `Integer` for
    // consistency.
    const EAGGREGATOR_OVERFLOW: u64 = 1;
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// Wrapper around integer to have a custom overflow limit. Note that
    /// Move has no traits (and trait bounds), so integer value must be u128.
    /// `Integer` provides API to add/subtract and read, just like `Aggregator`.
    struct Integer has store {
        value: u128,
        limit: u128,
    }

    fun new_integer(limit: u128): Integer {
        Integer {
            value: 0,
            limit,
        }
    }

    fun add_integer(integer: &mut Integer, value: u128) {
        assert!(
            value <= (integer.limit - integer.value),
            error::out_of_range(EAGGREGATOR_OVERFLOW)
        );
        integer.value = integer.value + value;
    }

    fun sub_integer(integer: &mut Integer, value: u128) {
        assert!(value <= integer.value, error::out_of_range(EAGGREGATOR_UNDERFLOW));
        integer.value = integer.value - value;
    }

    fun limit(integer: &Integer): u128 {
        integer.limit
    }

    fun read_integer(integer: &Integer): u128 {
        integer.value
    }

    fun destroy_integer(integer: Integer) {
        let Integer {value: _, limit: _ } = integer;
    }

    /// Struct that contains either an aggregator or a normal integer, both
    /// overflowing on limit.
    struct OptionalAggregator has store {
        // Parallelizable.
        aggregator: Option<Aggregator>,
        // Non-parallelizable.
        integer: Option<Integer>,
    }

    /// Creates a new optional aggregator instance.
    public(friend) fun new(limit: u128, parallelizable: bool): OptionalAggregator {
        if (parallelizable) {
            OptionalAggregator {
                aggregator: option::some(aggregator_factory::create_aggregator(limit)),
                integer: option::none(),
            }
        } else {
            OptionalAggregator {
                aggregator: option::none(),
                integer: option::some(new_integer(limit)),
            }
        }
    }

    /// Switches between parallelizable and non-parallelizable implementations.
    public(friend) fun switch(optional_aggregator: OptionalAggregator): OptionalAggregator {
        let value = read(&optional_aggregator);
        let new_optional_aggregator = switch_and_zero_out(optional_aggregator);
        add(&mut new_optional_aggregator, value);

        new_optional_aggregator
    }

    /// Switches between parallelizable and non-parallelizable implementations, setting
    /// the value of the new optional aggregator to zero.
    fun switch_and_zero_out(optional_aggregator: OptionalAggregator): OptionalAggregator {
        if (is_parallelizable(&optional_aggregator)) {
            // In this case we convert from Some(Agg), None to None, Some(Int).
            // First, get the limit and destroy old aggregator/integer pair.
            let limit = destroy_parallelizable(optional_aggregator);

            // Create a new instance of integer.
            OptionalAggregator {
                aggregator: option::none(),
                integer: option::some(new_integer(limit)),
            }
        } else {
            // Otherwise, it should be None, Some(Int) into Some(Agg), None.
            // Again, get the limit and destroy the old aggregator/integer first.
            let limit = destroy_non_parallelizable(optional_aggregator);

            // Create a new instance of aggregator.
            OptionalAggregator {
                aggregator: option::some(aggregator_factory::create_aggregator(limit)),
                integer: option::none(),
            }
        }
    }

    /// Destroys optional aggregator.
    public fun destroy(optional_aggregator: OptionalAggregator) {
        if (is_parallelizable(&optional_aggregator)) {
            destroy_parallelizable(optional_aggregator);
        } else {
            destroy_non_parallelizable(optional_aggregator);
        }
    }

    /// Destroys parallelizable optional aggregator and returns its limit.
    fun destroy_parallelizable(optional_aggregator: OptionalAggregator): u128 {
        let OptionalAggregator { aggregator, integer } = optional_aggregator;
        let limit = aggregator::limit(option::borrow(&aggregator));
        aggregator::destroy(option::destroy_some(aggregator));
        option::destroy_none(integer);
        limit
    }

    /// Destroys non-parallelizable optional aggregator and returns its limit.
    fun destroy_non_parallelizable(optional_aggregator: OptionalAggregator): u128 {
        let OptionalAggregator { aggregator, integer } = optional_aggregator;
        let limit = limit(option::borrow(&integer));
        destroy_integer(option::destroy_some(integer));
        option::destroy_none(aggregator);
        limit
    }

    /// Adds to optional aggregator, aborting on exceeding the `limit`.
    public fun add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow_mut(&mut optional_aggregator.aggregator);
            aggregator::add(aggregator, value);
        } else {
            let integer = option::borrow_mut(&mut optional_aggregator.integer);
            add_integer(integer, value);
        }
    }

    /// Subtracts from optional aggregator, aborting on going below zero.
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

    /// Returns true is optional aggregator uses parallelizable implementation.
    public fun is_parallelizable(optional_aggregator: &OptionalAggregator): bool {
        option::is_some(&optional_aggregator.aggregator)
    }

    #[test(account = @aptos_framework)]
    fun optional_aggregator_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);

        let aggregator = new(30, false);
        assert!(!is_parallelizable(&aggregator), 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 5, 0);

        // Switch to parallelizable aggregator and check the value is preserved.
        let aggregator = switch(aggregator);
        assert!(is_parallelizable(&aggregator), 0);
        assert!(read(&aggregator) == 5, 0);

        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 20, 0);

        sub(&mut aggregator, 10);
        assert!(read(&aggregator) == 10, 0);

        // Switch back!
        let aggregator = switch(aggregator);
        assert!(!is_parallelizable(&aggregator), 0);
        assert!(read(&aggregator) == 10, 0);

        destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    fun optional_aggregator_destriy_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);

        let aggregator = new(30, false);
        destroy(aggregator);

        let aggregator = new(30, true);
        destroy(aggregator);

        let aggregator = new(12, false);
        assert!(destroy_non_parallelizable(aggregator) == 12, 0);

        let aggregator = new(21, true);
        assert!(destroy_parallelizable(aggregator) == 21, 0);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020001)]
    fun non_parallelizable_aggregator_overflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(15, false);

        // Overflow!
        add(&mut aggregator, 16);

        destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020002)]
    fun non_parallelizable_aggregator_underflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(100, false);

        // Underflow!
        sub(&mut aggregator, 100);
        add(&mut aggregator, 100);

        destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020001)]
    fun parallelizable_aggregator_overflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(15, true);

        // Overflow!
        add(&mut aggregator, 16);

        destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020002)]
    fun parallelizable_aggregator_underflow_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);
        let aggregator = new(100, true);

        // Underflow!
        add(&mut aggregator, 99);
        sub(&mut aggregator, 100);
        add(&mut aggregator, 100);

        destroy(aggregator);
    }
}
