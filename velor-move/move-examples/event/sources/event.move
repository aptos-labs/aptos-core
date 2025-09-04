/// This provides an example shows how to use module events.

module event::event {
    use velor_framework::event;
    #[test_only]
    use std::vector;

    struct Field has store, drop {
        field: bool,
    }

    #[event]
    struct MyEvent has store, drop {
        seq: u64,
        field: Field,
        bytes: vector<u64>
    }

    public entry fun emit(num: u64) {
        let i = 0;
        while (i < num) {
            let event = MyEvent {
                seq: i,
                field: Field { field: false },
                bytes: vector[]
            };
            event::emit(event);
            i = i + 1;
        }
    }

    public entry fun call_inline() {
        emit_one_event()
    }

    inline fun emit_one_event() {
        event::emit(MyEvent {
            seq: 1,
            field: Field { field: false },
            bytes: vector[]
        });
    }

    #[test]
    public entry fun test_emitting() {
        emit(20);
        let module_events = event::emitted_events<MyEvent>();
        assert!(vector::length(&module_events) == 20, 0);
        let i = 0;
        while (i < 20) {
            let event = MyEvent {
                seq: i,
                field: Field {field: false},
                bytes: vector[]
            };
            assert!(vector::borrow(&module_events, i) == &event, i);
            i = i + 1;
        };
        let event = MyEvent {
            seq: 0,
            field: Field { field: false },
            bytes: vector[]
        };
        assert!(event::was_event_emitted(&event), i);
    }

    #[test]
    public entry fun test_inline() {
        call_inline();
        assert!(event::was_event_emitted(&MyEvent {
            seq: 1,
            field: Field { field: false },
            bytes: vector[]
        }), 0);
    }
}
