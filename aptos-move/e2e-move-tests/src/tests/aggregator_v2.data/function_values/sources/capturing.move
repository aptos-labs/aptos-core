module 0x1::capturing {
    use aptos_framework::aggregator_v2::create_unbounded_aggregator;

    public entry fun capture_aggregator() {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(1000);

        let apply = |x| 0x1::function_store::fetch_and_add(aggregator, x);
        let result = apply(100);
        assert!(result == 1100, 1);
    }

    public entry fun to_bytes_with_captured_aggregator() {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(1000);

        let apply = |x| 0x1::function_store::fetch_and_add(aggregator, x);
        let _ = 0x1::bcs::to_bytes(&apply);

        let result = apply(100);
        assert!(result == 1100, 2);
    }

    public entry fun serialized_size_with_captured_aggregator() {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(3);

        let apply = |x| 0x1::function_store::fetch_and_add(aggregator, x);
        let size = 0x1::bcs::serialized_size(&apply);
        assert!(size != 0, 4);

        let result = apply(100);
        assert!(result == 1100, 5);
    }

    public entry fun to_string_with_captured_aggregator() {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(1000);

        let apply = |x| 0x1::function_store::fetch_and_add(aggregator, x);
        let str = 0x1::string_utils::to_string(&apply);
        assert!(str != std::string::utf8(b"dummy"), 6);

        let result = apply(100);
        assert!(result == 1100, 7);
    }

    #[event]
    struct Event has store, drop {
        f: |u64|u64 has store + drop,
    }

    public entry fun emit_event_with_captured_aggregator() {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(1000);

        let f = |x| 0x1::function_store::fetch_and_add(aggregator, x);
        0x1::event::emit(Event { f });
    }
}
