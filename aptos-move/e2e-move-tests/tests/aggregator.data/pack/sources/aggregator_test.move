module 0x1::aggregator_test {
    use std::signer;

    use aptos_framework::aggregator::{Self, Aggregator};
    use aptos_framework::aggregator_factory;

    const ENOT_EQUAL: u64 = 17;

    struct AggregatorStore has key, store {
        aggregator: Aggregator,
    }

    public entry fun new(account: &signer, limit: u128) {
        let aggregator = aggregator_factory::create_aggregator_signed(account, limit);
        let store = AggregatorStore { aggregator };
        move_to(account, store);
    }

    public entry fun add(account: &signer, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregator = &mut borrow_global_mut<AggregatorStore>(addr).aggregator;
        aggregator::add(aggregator, value);
    }

    public entry fun sub(account: &signer, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregator = &mut borrow_global_mut<AggregatorStore>(addr).aggregator;
        aggregator::sub(aggregator, value);
    }

    public entry fun assert_eq(account: &signer, expected: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregator = &borrow_global<AggregatorStore>(addr).aggregator;
        let actual = aggregator::read(aggregator);
        assert!(actual == expected, ENOT_EQUAL)
    }
}
