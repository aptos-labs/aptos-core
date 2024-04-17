

module 0xABCD::aggregator_example {
    use std::error;
    use std::signer;
    use std::vector;
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

    struct FlagAggV2 has key {
        count: Aggregator<u64>,
    }

    struct BoundedAggV2Limit10 has key {
        count: Aggregator<u64>,
    }

    struct BoundedAggV2Limit100 has key {
        count: Aggregator<u64>,
    }

    struct BoundedAggV2Limit1000 has key {
        count: Aggregator<u64>,
    }

    struct AggregatorArrayCount1 has key {
        count: vector<Aggregator<u64>>,
    }

    struct AggregatorArrayCount10 has key {
        count: vector<Aggregator<u64>>,
    }

    struct AggregatorArrayCount100 has key {
        count: vector<Aggregator<u64>>,
    }

    struct AggregatorArrayCount1000 has key {
        count: vector<Aggregator<u64>>,
    }

    struct UnboundedAggV2 has key {
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
        move_to<FlagAggV2>(
            publisher,
            FlagAggV2 { count: aggregator_v2::create_aggregator(100) }
        );

        let agg = aggregator_v2::create_aggregator(10);
        aggregator_v2::try_add(&mut agg, 5);
        move_to<BoundedAggV2Limit10>(
            publisher,
            BoundedAggV2Limit10 { count: agg }
        );

        let agg2 = aggregator_v2::create_aggregator(100);
        aggregator_v2::try_add(&mut agg2, 50);
        move_to<BoundedAggV2Limit100>(
            publisher,
            BoundedAggV2Limit100 { count: agg2 }
        );

        let agg3 = aggregator_v2::create_aggregator(1000);
        aggregator_v2::try_add(&mut agg3, 500);
        move_to<BoundedAggV2Limit1000>(
            publisher,
            BoundedAggV2Limit1000 { count: agg3 }
        );

        let aggs1 = vector::empty();
        while (vector::length(&aggs1) < 1) {
            vector::push_back(&mut aggs1, aggregator_v2::create_unbounded_aggregator());
        };
        vector::push_back(&mut aggs1, aggregator_v2::create_unbounded_aggregator());
        move_to<AggregatorArrayCount1>(
            publisher,
            AggregatorArrayCount1 { count: aggs1 }
        );

        let aggs2 = vector::empty();
        while (vector::length(&aggs2) < 10) {
            vector::push_back(&mut aggs2, aggregator_v2::create_unbounded_aggregator());
        };
        move_to<AggregatorArrayCount10>(
            publisher,
            AggregatorArrayCount10 { count: aggs2 }
        );

        let aggs3 = vector::empty();
        while (vector::length(&aggs3) < 100) {
            vector::push_back(&mut aggs3, aggregator_v2::create_unbounded_aggregator());
        };
        move_to<AggregatorArrayCount100>(
            publisher,
            AggregatorArrayCount100 { count: aggs3 }
        );

        let aggs4 = vector::empty();
        while (vector::length(&aggs4) < 1000) {
            vector::push_back(&mut aggs4, aggregator_v2::create_unbounded_aggregator());
        };
        move_to<AggregatorArrayCount1000>(
            publisher,
            AggregatorArrayCount1000 { count: aggs4 }
        );
    }

    public entry fun increment() acquires Counter {
        assert!(exists<Counter>(@publisher_address), error::invalid_argument(ECOUNTER_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<Counter>(@publisher_address);
        *(&mut counter.count) = counter.count + 1;
    }

    public entry fun increment_agg_v2(count: u64) acquires CounterAggV2 {
        assert!(exists<CounterAggV2>(@publisher_address), error::invalid_argument(ECOUNTER_AGG_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<CounterAggV2>(@publisher_address);
        let i = 0;
        while (i < count) {
            aggregator_v2::try_add(&mut counter.count, 1);
            i = i + 1;
        }
    }

    public entry fun modify_bounded_agg_v2_limit_10(increment: bool, delta: u64) acquires BoundedAggV2Limit10 {
        assert!(exists<BoundedAggV2Limit10>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let bounded = borrow_global_mut<BoundedAggV2Limit10>(@publisher_address);
        if (increment) {
            aggregator_v2::try_add(&mut bounded.count, delta);
        } else {
            aggregator_v2::try_sub(&mut bounded.count, delta);
        }
    }

    public entry fun modify_bounded_agg_v2_limit_100(increment: bool, delta: u64) acquires BoundedAggV2Limit100 {
        assert!(exists<BoundedAggV2Limit100>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let bounded = borrow_global_mut<BoundedAggV2Limit100>(@publisher_address);
        if (increment) {
            aggregator_v2::try_add(&mut bounded.count, delta);
        } else {
            aggregator_v2::try_sub(&mut bounded.count, delta);
        }
    }

    public entry fun modify_bounded_agg_v2_limit_1000(increment: bool, delta: u64) acquires BoundedAggV2Limit1000 {
        assert!(exists<BoundedAggV2Limit1000>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let bounded = borrow_global_mut<BoundedAggV2Limit1000>(@publisher_address);
        if (increment) {
            aggregator_v2::try_add(&mut bounded.count, delta);
        } else {
            aggregator_v2::try_sub(&mut bounded.count, delta);
        }
    }
    
    public entry fun modify_flag_agg_v2(increment: bool, delta: u64) acquires FlagAggV2 {
        assert!(exists<FlagAggV2>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let bounded = borrow_global_mut<FlagAggV2>(@publisher_address);
        if (increment) {
            aggregator_v2::try_add(&mut bounded.count, delta);
        } else {
            aggregator_v2::try_sub(&mut bounded.count, delta);
        }
    }

    public entry fun modify_agg_array_count_1(increment: bool, delta: u64) acquires AggregatorArrayCount1 {
        assert!(exists<AggregatorArrayCount1>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let aggs = borrow_global_mut<AggregatorArrayCount1>(@publisher_address);
        if (increment) {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_add(agg, delta);
            });
        } else {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_sub(agg, delta);
            });
        }
    }

    public entry fun modify_agg_array_count_10(increment: bool, delta: u64) acquires AggregatorArrayCount10 {
        assert!(exists<AggregatorArrayCount10>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let aggs = borrow_global_mut<AggregatorArrayCount10>(@publisher_address);
        if (increment) {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_add(agg, delta);
            });
        } else {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_sub(agg, delta);
            });
        }
    }

    public entry fun modify_agg_array_count_100(increment: bool, delta: u64) acquires AggregatorArrayCount100 {
        assert!(exists<AggregatorArrayCount100>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let aggs = borrow_global_mut<AggregatorArrayCount100>(@publisher_address);
        if (increment) {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_add(agg, delta);
            });
        } else {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_sub(agg, delta);
            });
        }
    }

    public entry fun modify_agg_array_count_1000(increment: bool, delta: u64) acquires AggregatorArrayCount1000 {
        assert!(exists<AggregatorArrayCount1000>(@publisher_address), error::invalid_argument(EBOUNDED_AGG_RESOURCE_NOT_PRESENT));
        let aggs = borrow_global_mut<AggregatorArrayCount1000>(@publisher_address);
        if (increment) {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_add(agg, delta);
            });
        } else {
            vector::for_each_mut(&mut aggs.count, |agg| {
                aggregator_v2::try_sub(agg, delta);
            });
        }
    }
}
