module 0xABCD::aggregator_example {
    use std::error;
    use std::signer;
    use aptos_framework::aggregator_v2::{Self, Aggregator};

    // Resource being modified doesn't exist
    const ECOUNTER_RESOURCE_NOT_PRESENT: u64 = 1;

    // Resource being modified doesn't exist
    const ECOUNTER_AGG_RESOURCE_NOT_PRESENT: u64 = 2;

    // Resource being modified doesn't exist
    const EBOUNDED_AGG_RESOURCE_NOT_PRESENT: u64 = 3;

    // Incrementing a counter failed
    const ECOUNTER_INCREMENT_FAIL: u64 = 4;

    const ENOT_AUTHORIZED: u64 = 5;

    struct Counter has key {
        count: u64,
    }

    struct CounterAggV2 has key {
        count: Aggregator<u64>,
    }

    struct BoundedAggV2 has key {
        count: Aggregator<u64>,
    }

    // Create the global `Counter`.
    // Stored under the module publisher address.
    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @publisher_address,
            ENOT_AUTHORIZED,
        );

        move_to<Counter>(
            publisher,
            Counter { count: 0 }
        );
        move_to<CounterAggV2>(
            publisher,
            CounterAggV2 { count: aggregator_v2::create_unbounded_aggregator() }
        );
        move_to<BoundedAggV2>(
            publisher,
            BoundedAggV2 { count: aggregator_v2::create_aggregator(100) }
        );
    }

    public entry fun increment() acquires Counter {
        assert!(exists<Counter>(@publisher_address), error::invalid_argument(ECOUNTER_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<Counter>(@publisher_address);
        *(&mut counter.count) = counter.count + 1;
    }

    public entry fun increment_agg_v2() acquires CounterAggV2 {
        assert!(exists<CounterAggV2>(@publisher_address), error::invalid_argument(ECOUNTER_AGG_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<CounterAggV2>(@publisher_address);
        assert!(aggregator_v2::try_add(&mut counter.count, 1), ECOUNTER_INCREMENT_FAIL);
    }

    public entry fun modify_bounded_agg_v2(increment: bool, delta: u64) acquires BoundedAggV2 {
        assert!(exists<BoundedAggV2>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let bounded = borrow_global_mut<BoundedAggV2>(@publisher_address);
        if (increment) {
            aggregator_v2::try_add(&mut bounded.count, delta);
        } else {
            aggregator_v2::try_sub(&mut bounded.count, delta);
        }
    }
}
