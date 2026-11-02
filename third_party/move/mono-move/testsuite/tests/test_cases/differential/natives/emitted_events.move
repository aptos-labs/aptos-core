// RUN: publish
module 0x1::event {
    use std::vector;

    struct MyEvent has drop, store {
        a: u64,
        data: vector<u8>,
    }

    // Pointer-free, and padded: `a` at 0, `b` at 8, stride 16.
    struct Flat has drop, store {
        a: u8,
        b: u64,
    }

    struct Other has drop, store {
        x: u64,
    }

    struct ID has copy, drop, store {
        creation_num: u64,
        addr: address,
    }

    struct GUID has copy, drop, store {
        id: ID,
    }

    struct EventHandle<phantom T: drop + store> has drop, store {
        counter: u64,
        guid: GUID,
    }

    native fun write_module_event_to_store<T: drop + store>(msg: T);
    native fun write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T);
    native fun emitted_events<T: drop + store>(): vector<T>;
    native fun emitted_events_by_handle<T: drop + store>(handle: &EventHandle<T>): vector<T>;

    fun bytes(n: u8, first: u8): vector<u8> {
        let v = vector::empty<u8>();
        let i = 0u8;
        while (i < n) {
            vector::push_back(&mut v, first + i);
            i = i + 1;
        };
        v
    }

    fun push_bytes(out: &mut vector<u8>, data: &vector<u8>) {
        let i = 0;
        let n = vector::length(data);
        while (i < n) {
            vector::push_back(out, *vector::borrow(data, i));
            i = i + 1;
        }
    }

    // `[count, (a, len(data), data...)*]`, so one `vector<u8>` return carries
    // the whole query result.
    fun summarize(evs: &vector<MyEvent>): vector<u8> {
        let out = vector::empty<u8>();
        vector::push_back(&mut out, (vector::length(evs) as u8));
        let i = 0;
        while (i < vector::length(evs)) {
            let e = vector::borrow(evs, i);
            vector::push_back(&mut out, (e.a as u8));
            vector::push_back(&mut out, (vector::length(&e.data) as u8));
            push_bytes(&mut out, &e.data);
            i = i + 1;
        };
        out
    }

    // The legacy `write_module_event_to_store` checks that its *caller* is a
    // function in the event's module, so the native is never called from an
    // entry function -- those have no Move caller.
    fun emit_my(a: u64, data: vector<u8>) {
        write_module_event_to_store(MyEvent { a, data });
    }

    fun emit_flat(a: u8, b: u64) {
        write_module_event_to_store(Flat { a, b });
    }

    fun emit_other(x: u64) {
        write_module_event_to_store(Other { x });
    }

    // BCS of an `EventKey { creation_number, account_address }`: the counter
    // little-endian, then the address big-endian.
    fun guid_bytes(creation_num: u8, addr_byte: u8): vector<u8> {
        let g = vector::empty<u8>();
        vector::push_back(&mut g, creation_num);
        let i = 0;
        while (i < 7) {
            vector::push_back(&mut g, 0u8);
            i = i + 1;
        };
        let i = 0;
        while (i < 31) {
            vector::push_back(&mut g, 0u8);
            i = i + 1;
        };
        vector::push_back(&mut g, addr_byte);
        g
    }

    fun handle<T: drop + store>(creation_num: u64, addr: address): EventHandle<T> {
        EventHandle<T> { counter: 0, guid: GUID { id: ID { creation_num, addr } } }
    }

    public fun module_events(): vector<u8> {
        emit_my(7, bytes(3, 1));
        emit_my(9, bytes(2, 4));
        let evs = emitted_events<MyEvent>();
        summarize(&evs)
    }

    // Mutating the result must not reach the store: the payloads are deep
    // copies, so the events rendered after this call are the originals.
    public fun no_alias(): vector<u8> {
        emit_my(7, bytes(3, 1));
        let evs = emitted_events<MyEvent>();
        {
            let e = vector::borrow_mut(&mut evs, 0);
            *vector::borrow_mut(&mut e.data, 0) = 42u8;
            e.a = 100;
        };
        summarize(&evs)
    }

    public fun flat_events(): vector<u8> {
        emit_flat(1, 2);
        emit_flat(3, 4);
        emit_flat(5, 6);
        let evs = emitted_events<Flat>();
        let out = vector::empty<u8>();
        vector::push_back(&mut out, (vector::length(&evs) as u8));
        let i = 0;
        while (i < vector::length(&evs)) {
            let e = vector::borrow(&evs, i);
            vector::push_back(&mut out, e.a);
            vector::push_back(&mut out, (e.b as u8));
            i = i + 1;
        };
        out
    }

    // An empty payload is a null pointer slot, which must stay null.
    public fun empty_payload(): vector<u8> {
        emit_my(1, vector::empty<u8>());
        emit_my(2, bytes(2, 8));
        emit_my(3, vector::empty<u8>());
        let evs = emitted_events<MyEvent>();
        summarize(&evs)
    }

    public fun type_filter(): vector<u8> {
        emit_my(7, bytes(1, 1));
        emit_other(5);
        emit_my(8, bytes(1, 2));
        let mine = emitted_events<MyEvent>();
        let others = emitted_events<Other>();
        let out = summarize(&mine);
        vector::push_back(&mut out, (vector::length(&others) as u8));
        let i = 0;
        while (i < vector::length(&others)) {
            vector::push_back(&mut out, (vector::borrow(&others, i).x as u8));
            i = i + 1;
        };
        out
    }

    public fun empty_result(): vector<u8> {
        emit_my(1, bytes(1, 1));
        let evs = emitted_events<Other>();
        let out = vector::empty<u8>();
        vector::push_back(&mut out, (vector::length(&evs) as u8));
        out
    }

    // A handle (V1) event is invisible to `emitted_events`, and a module (V2)
    // event to `emitted_events_by_handle`.
    public fun kind_separation(): vector<u8> {
        emit_my(1, bytes(1, 1));
        write_to_event_store(guid_bytes(1, 0), 5u64, MyEvent { a: 2, data: bytes(1, 2) });
        let module_evs = emitted_events<MyEvent>();
        let h = handle<MyEvent>(1, @0x0);
        let handle_evs = emitted_events_by_handle(&h);
        let out = summarize(&module_evs);
        let tail = summarize(&handle_evs);
        push_bytes(&mut out, &tail);
        out
    }

    public fun handle_key_filter(): vector<u8> {
        write_to_event_store(guid_bytes(1, 0), 0u64, MyEvent { a: 1, data: bytes(1, 1) });
        write_to_event_store(guid_bytes(2, 0), 1u64, MyEvent { a: 2, data: bytes(1, 2) });
        write_to_event_store(guid_bytes(1, 0), 2u64, MyEvent { a: 3, data: bytes(1, 3) });
        write_to_event_store(guid_bytes(1, 7), 3u64, MyEvent { a: 4, data: bytes(1, 4) });
        let h1 = handle<MyEvent>(1, @0x0);
        let h2 = handle<MyEvent>(2, @0x0);
        let h3 = handle<MyEvent>(1, @0x7);
        let e1 = emitted_events_by_handle(&h1);
        let e2 = emitted_events_by_handle(&h2);
        let e3 = emitted_events_by_handle(&h3);
        let out = summarize(&e1);
        let s2 = summarize(&e2);
        let s3 = summarize(&e3);
        push_bytes(&mut out, &s2);
        push_bytes(&mut out, &s3);
        out
    }

    public fun many_events(): vector<u8> {
        let i = 0u64;
        while (i < 8) {
            emit_my(i, bytes(4, (i as u8)));
            i = i + 1;
        };
        let evs = emitted_events<MyEvent>();
        summarize(&evs)
    }
}

// RUN: execute 0x1::event::module_events
// CHECK: results: 0x02070301020309020405 | events: module 0x1::event::MyEvent 0x070000000000000003010203; module 0x1::event::MyEvent 0x0900000000000000020405

// RUN: execute 0x1::event::no_alias
// CHECK: results: 0x0164032a0203 | events: module 0x1::event::MyEvent 0x070000000000000003010203

// RUN: execute 0x1::event::flat_events
// CHECK: results: 0x03010203040506 | events: module 0x1::event::Flat 0x010200000000000000; module 0x1::event::Flat 0x030400000000000000; module 0x1::event::Flat 0x050600000000000000

// RUN: execute 0x1::event::empty_payload
// CHECK: results: 0x030100020208090300 | events: module 0x1::event::MyEvent 0x010000000000000000; module 0x1::event::MyEvent 0x0200000000000000020809; module 0x1::event::MyEvent 0x030000000000000000

// RUN: execute 0x1::event::type_filter
// CHECK: results: 0x020701010801020105 | events: module 0x1::event::MyEvent 0x07000000000000000101; module 0x1::event::Other 0x0500000000000000; module 0x1::event::MyEvent 0x08000000000000000102

// RUN: execute 0x1::event::empty_result
// CHECK: results: 0x00 | events: module 0x1::event::MyEvent 0x01000000000000000101

// RUN: execute 0x1::event::kind_separation
// CHECK: results: 0x0101010101020102 | events: module 0x1::event::MyEvent 0x01000000000000000101; handle creator=0x0 seq=5 0x1::event::MyEvent 0x02000000000000000102

// RUN: execute 0x1::event::handle_key_filter
// CHECK: results: 0x020101010301030102010201040104 | events: handle creator=0x0 seq=0 0x1::event::MyEvent 0x01000000000000000101; handle creator=0x0 seq=1 0x1::event::MyEvent 0x02000000000000000102; handle creator=0x0 seq=2 0x1::event::MyEvent 0x03000000000000000103; handle creator=0x7 seq=3 0x1::event::MyEvent 0x04000000000000000104

// Same query under collection pressure, so the rooting in the deep-copy batch
// is exercised against a real GC.
// RUN: execute 0x1::event::many_events --heap-size 704
// CHECK: results: 0x0800040001020301040102030402040203040503040304050604040405060705040506070806040607080907040708090a | events: module 0x1::event::MyEvent 0x00000000000000000400010203; module 0x1::event::MyEvent 0x01000000000000000401020304; module 0x1::event::MyEvent 0x02000000000000000402030405; module 0x1::event::MyEvent 0x03000000000000000403040506; module 0x1::event::MyEvent 0x04000000000000000404050607; module 0x1::event::MyEvent 0x05000000000000000405060708; module 0x1::event::MyEvent 0x06000000000000000406070809; module 0x1::event::MyEvent 0x0700000000000000040708090a
// CHECK-GC-COUNT: 1
