module 0x1::aggregator_v2_test {
    use aptos_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot};
    use aptos_std::table::{Self, Table};
    use std::signer;

    /// When checking the value of aggregator fails.
    const ENOT_EQUAL: u64 = 17;

    /// Resource to store aggregators. Each aggregator is associated with a
    /// determinictic integer value, for testing purposes.
    struct AggregatorStore has key, store {
        aggregators_u128: Table<u64, Aggregator<u128>>,
        aggregators_u64: Table<u64, Aggregator<u64>>,
        aggregator_snapshots_u128: Table<u64, AggregatorSnapshot<u128>>,
        aggregator_snapshots_u64: Table<u64, AggregatorSnapshot<u64>>,
    }
    
    /// Initializes a fake resource which holds aggregators.
    public entry fun initialize(account: &signer) {
        let aggregators_u128 = table::new();
        let aggregators_u64 = table::new();
        let aggregator_snapshots_u128 = table::new();
        let aggregator_snapshots_u64 = table::new();
        let store = AggregatorStore { aggregators_u128, aggregators_u64, aggregator_snapshots_u128, aggregator_snapshots_u64 };
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
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow(aggregators, i);
        let actual = aggregator_v2::read(aggregator);
        assert!(actual == expected, ENOT_EQUAL)
    }

    public entry fun new(account: &signer, i: u64, limit: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = aggregator_v2::create_aggregator(limit);
        table::add(aggregators, i, aggregator);
    }

    public entry fun try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_sub_add(account: &signer, i: u64, a: u128, b: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, a);
        aggregator_v2::try_add(aggregator, b);
    }

    public entry fun materialize(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow(aggregators, i);
        aggregator_v2::read(aggregator);
    }

    public entry fun materialize_and_try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun materialize_and_try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_add_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
        aggregator_v2::read(aggregator);
    }

    public entry fun try_sub_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
        aggregator_v2::read(aggregator);
    }

    public entry fun snapshot(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);

        let aggregator_u128 = table::borrow(&borrow_global<AggregatorStore>(addr).aggregators_u128, i);
        let aggregator_snapshot_u128 = aggregator_v2::snapshot(aggregator_u128);
        table::add(&mut borrow_global_mut<AggregatorStore>(addr).aggregator_snapshots_u128, i, aggregator_snapshot_u128);
        
        let aggregator_u64 = table::borrow(&borrow_global<AggregatorStore>(addr).aggregators_u64, i);
        let aggregator_snapshot_u64 = aggregator_v2::snapshot(aggregator_u64);
        table::add(&mut borrow_global_mut<AggregatorStore>(addr).aggregator_snapshots_u64, i, aggregator_snapshot_u64);
    }

    public entry fun read_snapshot_u128(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);

        let aggregator_snapshot_u128 = table::borrow(&borrow_global<AggregatorStore>(addr).aggregator_snapshots_u128, i);
        aggregator_v2::read_snapshot(aggregator_snapshot_u128);
    
        let aggregator_snapshot_u64 = table::borrow(&borrow_global<AggregatorStore>(addr).aggregator_snapshots_u64, i);
        aggregator_v2::read_snapshot(aggregator_snapshot_u64);
    }

    public entry fun try_add_snapshot(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators_u128;
        let aggregator = table::borrow_mut(aggregators, i);
        let aggregator_snapshot_1 = aggregator_v2::snapshot(aggregator);
        aggregator_v2::try_add(aggregator, value);
        let aggregator_snapshot_2 = aggregator_v2::snapshot(aggregator);
        aggregator_v2::try_add(aggregator, value);
        let aggregator_snapshot_3 = aggregator_v2::snapshot(aggregator);
        let snapshot_value_1 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_1);
        let snapshot_value_2 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_2);
        let snapshot_value_3 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_3);
        assert!(snapshot_value_2 == snapshot_value_1 + value, ENOT_EQUAL);
        assert!(snapshot_value_3 == snapshot_value_2 + value, ENOT_EQUAL);
    }
}
