module 0x1::events_with_aggregators {
    use std::signer;
    use velor_framework::account;
    use velor_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot};
    use velor_framework::event;

    const E_COUNTER_ALREADY_EXISTS: u64 = 0;
    const E_COUNTER_DOES_NOT_EXIST: u64 = 1;

    struct EventV1 has drop, store {
        value: AggregatorSnapshot<u64>,
    }

    #[event]
    struct EventV2 has drop, store {
        value: AggregatorSnapshot<u64>,
    }

    struct EventStream has key {
        events_v1: event::EventHandle<EventV1>,
    }

    struct Counter has key {
        counter: Aggregator<u64>,
    }

    fun init_module(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<Counter>(addr), E_COUNTER_ALREADY_EXISTS);

        let counter = Counter {
            counter: aggregator_v2::create_aggregator(10),
        };
        move_to(account, counter);

        let event_stream = EventStream {
            events_v1: account::new_event_handle(account)
        };
        move_to(account, event_stream);
    }

    public entry fun increment_counter(addr: address) acquires Counter {
        assert!(exists<Counter>(addr), E_COUNTER_DOES_NOT_EXIST);
        let counter = &mut borrow_global_mut<Counter>(addr).counter;
        aggregator_v2::add(counter, 1);
    }

    public entry fun test_increment_counter_and_emit_event_v1(addr: address) acquires Counter, EventStream {
        increment_counter(addr);
        test_emit_event_v1(addr);
    }

    public entry fun test_increment_counter_and_emit_event_v2(addr: address) acquires Counter {
        increment_counter(addr);
        test_emit_event_v2(addr);
    }

    public entry fun test_emit_event_v1(addr: address) acquires Counter, EventStream {
        assert!(exists<Counter>(addr), E_COUNTER_DOES_NOT_EXIST);
        let counter = &borrow_global<Counter>(addr).counter;
        let event = EventV1 {
            value: aggregator_v2::snapshot(counter),
        };
        let event_stream = &mut borrow_global_mut<EventStream>(addr).events_v1;
        event::emit_event(event_stream, event);
    }

    public entry fun test_emit_event_v2(addr: address) acquires Counter {
        assert!(exists<Counter>(addr), E_COUNTER_DOES_NOT_EXIST);
        let counter = &borrow_global<Counter>(addr).counter;
        let event = EventV2 {
            value: aggregator_v2::snapshot(counter),
        };
        event::emit(event);
    }
}
