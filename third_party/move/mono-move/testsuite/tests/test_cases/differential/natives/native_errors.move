// Natives that fail with a VM error rather than an abort, on inputs the test's own modules
// build: a String that is not UTF-8, a forged ristretto255 point handle (V1 aborts instead),
// a short ristretto255 scalar, a forged crypto_algebra handle, and module events emitted from
// the entry function, by a module that does not define the event type, or of a non-struct
// type. The VMs fail differently, so each side is pinned (V1 by substring where its message
// carries a backtrace). Emitting from the module that defines the event type succeeds on both.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }
    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;
    native fun scalar_invert_internal(bytes: vector<u8>): vector<u8>;

    public fun forged_handle(): vector<u8> {
        point_compress_internal(&RistrettoPoint { handle: 77 })
    }

    public fun short_scalar(): vector<u8> {
        scalar_invert_internal(x"01")
    }
}
module 0x1::bls12381_algebra {
    struct Fr {}
    struct FormatFrLsb {}
}
module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra::{Fr, FormatFrLsb};
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    public fun forged_handle(): vector<u8> {
        serialize_internal<Fr, FormatFrLsb>(5)
    }
}
module 0x1::event {
    struct MyEvent has drop, store {
        a: u64,
    }
    native fun write_module_event_to_store<T: drop + store>(msg: T);

    public fun emit_from_entry() {
        write_module_event_to_store(MyEvent { a: 1 });
    }

    public fun make(): MyEvent {
        MyEvent { a: 2 }
    }

    public fun emit<T: drop + store>(msg: T) {
        write_module_event_to_store(msg);
    }
}
module 0x42::other {
    struct Foreign has drop, store {
        a: u64,
    }

    // Allowed: the caller of `emit` defines the event type.
    public fun emit_foreign() {
        0x1::event::emit(Foreign { a: 3 });
    }

    // The caller of `emit` does not define the event type.
    public fun emit_misowned() {
        0x1::event::emit(0x1::event::make());
    }

    public fun emit_u64() {
        0x1::event::emit(5u64);
    }
}
module 0x1::main {
    use std::string;

    public fun index_of_invalid(): u64 {
        let text = 0x1::from_bcs::from_bytes<string::String>(x"01ff");
        string::index_of(&text, &string::utf8(b"a"))
    }

    public fun sub_string_invalid(): u64 {
        let text = 0x1::from_bcs::from_bytes<string::String>(x"02ffff");
        string::length(&string::sub_string(&text, 0, 1))
    }
}


// RUN: execute 0x1::main::index_of_invalid
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("string") } and message Every string must be UTF-8
// CHECK-V2: error: native function invariant violation: Every string must be UTF-8

// RUN: execute 0x1::main::sub_string_invalid
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("string") } and message Every string must be UTF-8
// CHECK-V2: error: native function invariant violation: Every string must be UTF-8

// RUN: execute 0x1::ristretto255::forged_handle
// CHECK-V1: aborted: code 655361 (Invalid point handle) in 0x1::ristretto255
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 77

// RUN: execute 0x1::ristretto255::short_scalar
// CHECK-V1: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255") } at code offset 0 in function definition 2
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::crypto_algebra::forged_handle
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("crypto_algebra") } and message aptos_std::crypto_algebra native abort
// CHECK-V2: error: native function invariant violation: invalid crypto_algebra handle: 5

// RUN: execute 0x1::event::emit_from_entry
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("event") } and message Caller frame for 0x1::emit::event is not found
// CHECK-V2: error: native function invariant violation: write_module_event_to_store: scripts cannot emit module events

// RUN: execute 0x42::other::emit_foreign
// CHECK: events: module 0x42::other::Foreign 0x0300000000000000

// RUN: execute 0x42::other::emit_misowned
// CHECK-V1: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("event") } at code offset 0 in function definition 3
// CHECK-V2: error: native function invariant violation: write_module_event_to_store: caller module does not define the event type

// RUN: execute 0x42::other::emit_u64
// CHECK-V1: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("event") } at code offset 0 in function definition 3
// CHECK-V2: error: native function invariant violation: write_module_event_to_store: event type must be a struct or enum
