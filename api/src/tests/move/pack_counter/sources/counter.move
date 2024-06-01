module addr::counter {
    use 0x1::aggregator_v2::{Self, Aggregator};

    struct Counter has key {
        counter: Aggregator<u64>,
    }

    fun init_module(account: &signer) {
        let counter = Counter {
            counter: aggregator_v2::create_aggregator(100),
        };
        move_to(account, counter);
    }

    public entry fun increment_counter() acquires Counter {
        let counter = &mut borrow_global_mut<Counter>(@addr).counter;
        aggregator_v2::add(counter, 1);
    }

    #[view]
    public fun add_and_get_counter_value(): u64 acquires Counter {
        let counter = &mut borrow_global_mut<Counter>(@addr).counter;
        aggregator_v2::add(counter, 10);
        aggregator_v2::read(counter)
    }
}
