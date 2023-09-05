module 0x1::aggregator_v2_test {
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_std::table::{Self, Table};
    use std::signer;

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

    public entry fun verify_copy_snapshot() {
        let snapshot = aggregator_v2::create_snapshot(42);
        let snapshot2 = aggregator_v2::copy_snapshot(&snapshot);
        assert!(aggregator_v2::read_snapshot(&snapshot) == 42, 0);
        assert!(aggregator_v2::read_snapshot(&snapshot2) == 42, 0);
    }

    public entry fun verify_copy_string_snapshot() {
        let snapshot = aggregator_v2::create_snapshot(std::string::utf8(b"42"));
        let snapshot2 = aggregator_v2::copy_snapshot(&snapshot);
        assert!(aggregator_v2::read_snapshot(&snapshot) == std::string::utf8(b"42"), 0);
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"42"), 0);
    }

    public entry fun verify_string_concat() {
        let snapshot = aggregator_v2::create_snapshot(42);
        let snapshot2 = aggregator_v2::string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }

    public entry fun verify_string_snapshot_concat() {
        let snapshot = aggregator_v2::create_snapshot(std::string::utf8(b"42"));
        let snapshot2 = aggregator_v2::string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }


    /// Checks that the ith aggregator has expected value. Useful to inject into
    /// transaction block to verify successful and correct execution.
    public entry fun check(account: &signer, i: u64, expected: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        let actual = aggregator_v2::read(aggregator);
        assert!(actual == expected, ENOT_EQUAL)
    }

    public entry fun new(account: &signer, i: u64, limit: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = aggregator_v2::create_aggregator(limit);
        table::add(aggregators, i, aggregator);
    }

    public entry fun try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_sub_add(account: &signer, i: u64, a: u128, b: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, a);
        aggregator_v2::try_add(aggregator, b);
    }

    public entry fun materialize(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        aggregator_v2::read(aggregator);
    }

    public entry fun materialize_and_try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun materialize_and_try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_add_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
        aggregator_v2::read(aggregator);
    }

    public entry fun try_sub_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
        aggregator_v2::read(aggregator);
    }
}
