// `write_module_event_to_store` and `get_caller_address_and_module_id` look at the module that
// called the native's Move caller. Each is reached through a lambda defined in one module and
// invoked from another: when the lambda calls the Move wrapper (`emit`, `native_hash`) the
// module seen is the lambda's, and when it calls the native directly it is the invoker's.
// The VMs reject with different errors; V1's location names a compiler-assigned function
// index, so it is matched only up to the module.

// RUN: publish
module 0x1::event {
    #[event]
    struct Direct has drop, store {
        value: u64,
    }

    native fun write_module_event_to_store<T: drop + store>(msg: T);

    public fun emit<T: drop + store>(msg: T) {
        write_module_event_to_store(msg);
    }

    public fun direct_emitter(): |u64| has drop {
        |value| write_module_event_to_store(Direct { value })
    }

    public fun invoke_direct_here(value: u64) {
        let emit_direct = direct_emitter();
        emit_direct(value)
    }
}

module 0x1::init {
    struct ModuleId has copy, drop, store {
        hash: u128
    }

    native fun get_caller_address_and_module_id(): (address, ModuleId);

    public fun native_hash(): (address, u128) {
        let (addr, id) = get_caller_address_and_module_id();
        (addr, id.hash)
    }
}

module 0x42::producer {
    #[event]
    struct Ev has drop, store {
        value: u64,
    }

    public fun emitter(): |u64| has drop {
        |value| 0x1::event::emit(Ev { value })
    }

    public fun make(value: u64): Ev {
        Ev { value }
    }
}

module 0x42::lender {
    // The offset keeps the body from reducing to a closure over `native_hash`.
    public fun hasher(): |u128| (address, u128) has drop {
        |offset| {
            let (addr, hash) = 0x1::init::native_hash();
            (addr, hash + offset)
        }
    }
}

module 0x42::consumer {
    public fun invoke_producer_lambda(value: u64) {
        let emit_ev = 0x42::producer::emitter();
        emit_ev(value)
    }

    public fun emit_producer_event(value: u64) {
        0x1::event::emit(0x42::producer::make(value))
    }

    public fun invoke_direct_lambda(value: u64) {
        let emit_direct = 0x1::event::direct_emitter();
        emit_direct(value)
    }
}

module 0x43::borrower {
    public fun invoke_lender_lambda(): (address, u128) {
        let hash_of_caller = 0x42::lender::hasher();
        hash_of_caller(0)
    }
}

// RUN: execute 0x42::consumer::invoke_producer_lambda --args 4
// CHECK: events: module 0x42::producer::Ev 0x0400000000000000

// RUN: execute 0x42::consumer::emit_producer_event --args 4
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("event") }
// CHECK-V2: error: native function invariant violation: write_module_event_to_store: caller module does not define the event type

// RUN: execute 0x1::event::invoke_direct_here --args 5
// CHECK: events: module 0x1::event::Direct 0x0500000000000000

// RUN: execute 0x42::consumer::invoke_direct_lambda --args 5
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("event") }
// CHECK-V2: error: native function invariant violation: write_module_event_to_store: caller module does not define the event type

// sha3-256("lender"), first 16 bytes read as a little-endian u128.
// RUN: execute 0x43::borrower::invoke_lender_lambda
// CHECK: results: 0x42, 234309523122817895450889064239666668635
