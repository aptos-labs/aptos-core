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

    fun add_integer(base: &mut Integer, value: u128) {
        assert!(
            value <= (base.limit - base.value),
            error::out_of_range(EAGGREGATOR_OVERFLOW)
        );
        base.value = base.value + value;
    }

    fun sub_integer(base: &mut Integer, value: u128) {
        assert!(value <= base.value, error::out_of_range(EAGGREGATOR_UNDERFLOW));
        base.value = base.value - value;
    }

    fun read_integer(base: &Integer): u128 {
        base.value
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

    public fun new(limit: u128, parallelizable: bool): OptionalAggregator {
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

    public fun destroy(optional_aggregator: OptionalAggregator) {
        let OptionalAggregator { aggregator, integer } = optional_aggregator;

        if (option::is_some(&aggregator)) {
            aggregator::destroy(option::destroy_some(aggregator));
            option::destroy_none(integer);
        } else {
            destroy_integer(option::destroy_some(integer));
            option::destroy_none(aggregator);
        }
    }

    public fun add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow_mut(&mut optional_aggregator.aggregator);
            aggregator::add(aggregator, value);
        } else {
            let integer = option::borrow_mut(&mut optional_aggregator.integer);
            add_integer(integer, value);
        }
    }

    public fun sub(optional_aggregator: &mut OptionalAggregator, value: u128) {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow_mut(&mut optional_aggregator.aggregator);
            aggregator::sub(aggregator, value);
        } else {
            let integer = option::borrow_mut(&mut optional_aggregator.integer);
            sub_integer(integer, value);
        }
    }

    public fun read(optional_aggregator: &OptionalAggregator): u128 {
        if (option::is_some(&optional_aggregator.aggregator)) {
            let aggregator = option::borrow(&optional_aggregator.aggregator);
            aggregator::read(aggregator)
        } else {
            let integer = option::borrow(&optional_aggregator.integer);
            read_integer(integer)
        }
    }

    #[test(account = @aptos_framework)]
    fun optional_aggregator_test(account: signer) {
        aggregator_factory::initialize_aggregator_factory(&account);

        let aggregator = new(15, false);
        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        sub(&mut aggregator, 15);
        assert!(read(&aggregator) == 0, 0);
        destroy(aggregator);

        // Repate with parallelizable aggregator.
        let aggregator = new(15, true);
        add(&mut aggregator, 12);
        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        sub(&mut aggregator, 15);
        assert!(read(&aggregator) == 0, 0);
        destroy(aggregator);
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
