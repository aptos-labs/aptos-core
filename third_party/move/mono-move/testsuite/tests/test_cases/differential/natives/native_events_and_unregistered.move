// Module events of an enum type, and emitted from a lambda in the event's module, succeed
// on both VMs. A native the test declares but neither VM provides fails on both, with
// different errors: V1 with MISSING_DEPENDENCY at the call, MonoMove when loading it.
// `read_u8` calls `aggregator_v2::read` on a u8 aggregator built from the test's own struct
// (the framework cannot build one); it is pinned on MonoMove only, which has no native for
// that instance.

// RUN: publish
module 0x1::event {
    enum Kind has drop, store {
        A { x: u64 },
        B,
    }

    struct MyEvent has drop, store {
        a: u64,
    }

    native fun write_module_event_to_store<T: drop + store>(msg: T);

    fun emit_enum() {
        write_module_event_to_store(Kind::A { x: 9 });
        write_module_event_to_store(Kind::B);
    }

    public fun enum_event() {
        emit_enum();
    }

    fun emit_via_lambda() {
        let func: |u64| has drop = |value| write_module_event_to_store(MyEvent { a: value });
        func(4);
    }

    public fun lambda_event() {
        emit_via_lambda();
    }
}
module 0x1::aggregator_v2 {
    struct Aggregator<IntElement> has store, drop {
        value: IntElement,
        max_value: IntElement,
    }
    public native fun read<IntElement>(self: &Aggregator<IntElement>): IntElement;

    public fun read_u8(): u8 {
        read(&Aggregator<u8> { value: 1, max_value: 2 })
    }
}
module 0x1::nowhere {
    native fun missing(value: u64): u64;

    public fun call_missing(): u64 {
        missing(1)
    }
}
// RUN: execute 0x1::event::enum_event
// CHECK: events: module 0x1::event::Kind 0x000900000000000000; module 0x1::event::Kind 0x01

// RUN: execute 0x1::event::lambda_event
// CHECK: events: module 0x1::event::MyEvent 0x0400000000000000

// RUN: execute 0x1::aggregator_v2::read_u8
// CHECK-V2: error: Function 0x1::aggregator_v2::read is a native and cannot be loaded as code

// RUN: execute 0x1::nowhere::call_missing
// CHECK-V1: error: VMError with status MISSING_DEPENDENCY at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("nowhere") } and message Missing Native Function `missing` at code offset 0 in function definition 1
// CHECK-V2: error: Function 0x1::nowhere::missing is a native and cannot be loaded as code
