// A committing step that emits a module event and writes a resource: the step's output shows
// both the event and the write, and the resource is readable in the next step.

// RUN: publish
module 0x1::event {
    struct Note has drop, store {
        data: vector<u8>,
    }

    struct Holder has key { v: u64 }

    native fun write_module_event_to_store<T: drop + store>(msg: T);

    fun emit_note() {
        write_module_event_to_store(Note { data: b"hi" });
    }

    public fun emit_and_store(account: signer) {
        emit_note();
        move_to(&account, Holder { v: 3 });
    }

    public fun read(addr: address): u64 acquires Holder {
        borrow_global<Holder>(addr).v
    }
}

// RUN: execute 0x1::event::emit_and_store --args 0x5 --commit
// CHECK: events: module 0x1::event::Note 0x026869 | writes: create 0x1::event::Holder at 0x5 0x0300000000000000

// RUN: execute 0x1::event::read --args 0x5
// CHECK: results: 3
