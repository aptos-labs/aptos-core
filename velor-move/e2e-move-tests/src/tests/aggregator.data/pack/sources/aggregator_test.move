module 0x1::aggregator_test {
    use std::signer;

    use velor_framework::aggregator::{Self, Aggregator};
    use velor_framework::aggregator_factory;
    use velor_std::table::{Self, Table};

    /// When checking the value of aggregator fails.
    const ENOT_EQUAL: u64 = 17;

    /// Resource to store aggregators. Each aggregator is associated with a
    /// determinictic integer value, for testing purposes.
    struct AggregatorStore has key, store {
        aggregators: Table<u64, Aggregator>,
    }

    /// Initializes a fake resource which holds aggregators.
    public entry fun initialize(account: &signer) {
        let aggregators = table::new();
        let store = AggregatorStore { aggregators };
        move_to(account, store);
    }

    /// Checks that the ith aggregator has expected value. Useful to inject into
    /// transaction block to verify successful and correct execution.
    public entry fun check(account: &signer, i: u64, expected: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        let actual = aggregator::read(aggregator);
        assert!(actual == expected, ENOT_EQUAL)
    }

    //
    // Testing scripts.
    //

    public entry fun new(account: &signer, i: u64, limit: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = aggregator_factory::create_aggregator(account, limit);
        table::add(aggregators, i, aggregator);
    }

    public entry fun add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::add(aggregator, value);
    }

    public entry fun sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::sub(aggregator, value);
    }

    public entry fun sub_add(account: &signer, i: u64, a: u128, b: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::sub(aggregator, a);
        aggregator::add(aggregator, b);
    }

    public entry fun destroy(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;

        let aggregator = table::remove(aggregators, i);
        aggregator::destroy(aggregator);
    }

    public entry fun materialize(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        aggregator::read(aggregator);
    }

    public entry fun materialize_and_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::read(aggregator);
        aggregator::add(aggregator, value);
    }

    public entry fun materialize_and_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::read(aggregator);
        aggregator::sub(aggregator, value);
    }

    public entry fun add_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::add(aggregator, value);
        aggregator::read(aggregator);
    }

    public entry fun sub_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator::sub(aggregator, value);
        aggregator::read(aggregator);
    }
}
