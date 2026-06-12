// Differential test for the event natives `write_module_event_to_store` (V2,
// module events) and `write_to_event_store` (V1, handle events).

// RUN: publish
module 0x1::event {
    use std::vector;

    struct MyEvent has drop, store {
        a: u64,
        data: vector<u8>,
    }

    native fun write_module_event_to_store<T: drop + store>(msg: T);
    native fun write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T);

    fun bytes_1_2_3(): vector<u8> {
        let v = vector::empty<u8>();
        vector::push_back(&mut v, 1u8);
        vector::push_back(&mut v, 2u8);
        vector::push_back(&mut v, 3u8);
        v
    }

    // The legacy `write_module_event_to_store` checks that its *caller* is a
    // function in the event's module, so the native is called from a nested
    // function.
    //
    // The entry function itself is not sufficient -- it does not have a Move
    // *caller*.
    fun do_emit() {
        write_module_event_to_store(MyEvent { a: 7, data: bytes_1_2_3() });
    }

    // Module (V2) event.
    public fun emit_module() {
        do_emit();
    }

    // Mock `guid` -- the BCS encoding of an `EventKey { creation_number: 1,
    // account_address: 0x0 }`
    fun handle_guid(): vector<u8> {
        let guid = vector::empty<u8>();
        vector::push_back(&mut guid, 1u8);
        let i = 0;
        while (i < 39) {
            vector::push_back(&mut guid, 0u8);
            i = i + 1;
        };
        guid
    }

    // Handle (V1) event.
    public fun emit_handle() {
        write_to_event_store(handle_guid(), 5u64, MyEvent { a: 9, data: bytes_1_2_3() });
    }
}

// RUN: execute 0x1::event::emit_module
// CHECK: events: module 0x1::event::MyEvent 0x070000000000000003010203

// RUN: execute 0x1::event::emit_handle
// CHECK: events: handle creator=0x0 seq=5 0x1::event::MyEvent 0x090000000000000003010203
