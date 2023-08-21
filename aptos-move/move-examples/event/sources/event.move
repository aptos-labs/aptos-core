/// This provides an example shows how to use module events.

module event::event {
    use aptos_framework::event;
    #[test_only]
    use std::vector;

    struct Field has store, drop {}

    #[event]
    struct MyEvent has store, drop {
        seq: u64,
        field: Field,
        bytes: vector<u64>
    }

    public fun emit(num: u64) {
        let i = 0;
        while (i < num) {
            let event = MyEvent {
                seq: i,
                field: Field {},
                bytes: vector[]
            };
            event::emit(&event);
            i = i + 1;
        }
    }

    #[test]
    public fun test_emitting() {
        emit(20);
        let module_events = event::emitted_events<MyEvent>();
        assert!(vector::length(&module_events) == 20, 0);
        let i = 0;
        while (i < 20) {
            let event = MyEvent {
                seq: i,
                field: Field {},
                bytes: vector[]
            };
            assert!(vector::borrow(&module_events, i) == &event, i);
            i = i + 1;
        };
        let event = MyEvent {
            seq: 0,
            field: Field {},
            bytes: vector[]
        };
        assert!(event::was_event_emitted(&event), i);
    }
}
