module 0x815::test_module {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::event;

    struct EventV1<phantom T> has store, drop {}

    struct EventV1StreamOk has key {
        events: event::EventHandle<EventV1<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>,
    }

    struct EventV1StreamErr has key {
        // Same as EventV1StreamOk, but has one more nested vector.
        events: event::EventHandle<EventV1<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>>,
    }


    #[event]
    struct EventV2<phantom T> has store, drop{}

    fun init_module(account: &signer) {
        let event_stream_ok = EventV1StreamOk {
            events: account::new_event_handle(account)
        };
        move_to(account, event_stream_ok);
        let event_stream_err = EventV1StreamErr {
            events: account::new_event_handle(account)
        };
        move_to(account, event_stream_err);
    }

    public entry fun emit_event_v1_ok(account: &signer) acquires EventV1StreamOk {
        let addr = signer::address_of(account);
        let event_stream = &mut borrow_global_mut<EventV1StreamOk>(addr).events;

        let event =  EventV1<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>{};
        event::emit_event(event_stream, event);
    }

    public entry fun emit_event_v1_too_large(account: &signer) acquires EventV1StreamErr {
        let addr = signer::address_of(account);
        let event_stream = &mut borrow_global_mut<EventV1StreamErr>(addr).events;

        let event =  EventV1<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>{};
        event::emit_event(event_stream, event);
    }

    public entry fun emit_event_v2_ok(_account: &signer){
        let event =  EventV2<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>{};
        event::emit(event);
    }

    public entry fun emit_event_v2_too_large(_account: &signer){
        // Same as in ok case, but with one extra nested vector.
        let event =  EventV2<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>{};
        event::emit(event);
    }
}
