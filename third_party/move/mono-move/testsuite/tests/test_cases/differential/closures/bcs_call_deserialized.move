// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}

module 0x99::closure_bcs {
    use std::bcs;

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    fun private_add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun pick<T: drop>(a: T, b: u64): u64 {
        let _ = a;
        b
    }

    struct NoDrop has copy {
        x: u64,
    }

    #[persistent]
    fun who(s: signer): address {
        std::signer::address_of(&s)
    }

    #[persistent]
    fun mix(a: u8, b: u64, c: vector<u8>): u64 {
        (a as u64) + b + c.length()
    }

    enum Shape has copy, drop, store {
        Circle { r: u64 },
        Rect { w: u64, h: u64 },
    }

    #[persistent]
    fun area(s: Shape, k: u64): u64 {
        match (s) {
            Shape::Circle { r } => r * r + k,
            Shape::Rect { w, h } => w * h + k,
        }
    }

    #[persistent]
    fun nested(v: vector<vector<u8>>, k: u64): u64 {
        v.length() + v[1].length() + k
    }

    const FILLER: vector<u64> = vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    fun wire(f: &|u64|u64 has copy + drop + store): |u64|u64 has copy + drop + store {
        0x1::from_bcs::from_bytes<|u64|u64 has copy + drop + store>(bcs::to_bytes(f))
    }

    // Leaves FILLER as dead heap once this frame is popped.
    fun fill(): u64 {
        let filler = FILLER;
        filler.length()
    }

    public fun call_noncapturing(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|u64 has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000");
        f(3, 4)
    }

    public fun call_capturing(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        f(5)
    }

    public fun call_vector_capture(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        f(10)
    }

    public fun call_both_copies(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        let g = f;
        f(1) + g(2)
    }

    public fun call_generic_target(): u64 {
        let g: |u64|u64 has copy + drop + store = |b| pick<u8>(9, b);
        let f = wire(&g);
        f(4)
    }

    public fun call_mask_gap(): u64 {
        let c = vector[2u8, 3u8];
        let g: |u64|u64 has copy + drop + store = |b| mix(1, b, c);
        let f = wire(&g);
        f(10)
    }

    public fun call_enum_capture(): u64 {
        let s = Shape::Rect { w: 2, h: 3 };
        let g: |u64|u64 has copy + drop + store = |k| area(s, k);
        let f = wire(&g);
        f(1)
    }

    public fun call_nested_vector_capture(): u64 {
        let v = vector[vector[1u8], vector[2u8, 3u8]];
        let g: |u64|u64 has copy + drop + store = |k| nested(v, k);
        let f = wire(&g);
        f(1)
    }

    public fun call_raw_closures_in_vector_across_gc(): u64 {
        let a: |u64|u64 has copy + drop + store = |b| add(7, b);
        let d = vector[1u8, 2u8];
        let t: |u64|u64 has copy + drop + store = |k| tally(d, k);
        let v = vector[wire(&a), wire(&t)];
        0x0::test_utils::force_gc();
        let f = v[0];
        let g = v[1];
        f(1) + g(1)
    }

    public fun trailing_bytes(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730361646400010000000000000002070000000000000000");
        f(1)
    }

    public fun call_missing_module(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990e6e6f5f737563685f6d6f64756c6503616464000100000000000000020700000000000000");
        f(1)
    }

    public fun call_missing_function(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000");
        f(1)
    }

    // The bytes name `private_add`, which is not persistent.
    public fun call_non_persistent_target(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730b707269766174655f616464000100000000000000020700000000000000");
        f(1)
    }

    // The mask captures a third parameter `add` does not have.
    public fun call_mask_beyond_params(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000400000000000000020700000000000000");
        f(1, 2)
    }

    // The bytes give `add` a type argument it does not take.
    public fun call_wrong_type_arg_count(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730361646401020100000000000000020700000000000000");
        f(1)
    }

    // The capture was stored as a `bool`, but `add` takes a `u64`.
    public fun call_capture_layout_mismatch(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373036164640001000000000000000001");
        f(1)
    }

    // The bytes instantiate `pick` with `NoDrop`, which lacks the `drop` its
    // type parameter requires.
    public fun call_unsatisfied_constraint(): u64 {
        let f = 0x1::from_bcs::from_bytes<|NoDrop, u64|u64 has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373047069636b010700000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373064e6f44726f70000000000000000000");
        f(NoDrop { x: 1 }, 2)
    }

    // The bytes name `who`, which takes a `signer`, as a function taking an
    // `address`.
    public fun call_param_type_mismatch(): address {
        let f = 0x1::from_bcs::from_bytes<|address|address has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730377686f000000000000000000");
        f(@0x42)
    }

    // The bytes name `add`, which returns one value, as a function returning
    // two.
    public fun call_result_count_mismatch(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|(u64, u64) has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000");
        let (a, b) = f(1, 2);
        a + b
    }

    // `call_with_gc_while_decoding` up to the call.
    public fun decode_and_fill(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        let _ = f;
        fill()
    }

    public fun call_with_gc_while_decoding(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        let n = fill();
        f(n)
    }

    public fun call_across_gc(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        0x0::test_utils::force_gc();
        let a = f(10);
        0x0::test_utils::force_gc();
        let b = f(20);
        a + b
    }
}

// RUN: execute 0x99::closure_bcs::call_noncapturing
// CHECK: results: 7

// RUN: execute 0x99::closure_bcs::call_capturing
// CHECK: results: 12

// RUN: execute 0x99::closure_bcs::call_vector_capture
// CHECK: results: 13

// RUN: execute 0x99::closure_bcs::call_across_gc
// CHECK: results: 36

// RUN: execute 0x99::closure_bcs::call_both_copies
// CHECK: results: 17

// RUN: execute 0x99::closure_bcs::call_generic_target
// CHECK: results: 4

// RUN: execute 0x99::closure_bcs::call_mask_gap
// CHECK: results: 13

// RUN: execute 0x99::closure_bcs::call_enum_capture
// CHECK: results: 7

// RUN: execute 0x99::closure_bcs::call_nested_vector_capture
// CHECK: results: 5

// RUN: execute 0x99::closure_bcs::call_raw_closures_in_vector_across_gc
// CHECK: results: 11

// RUN: execute 0x99::closure_bcs::trailing_bytes
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x99::closure_bcs::call_missing_module
// CHECK-ERROR-PARITY

// Both fail to link; MonoMove does not report V1's FUNCTION_RESOLUTION_FAILURE.
// RUN: execute 0x99::closure_bcs::call_missing_function
// CHECK-V1-SUBSTR: closure_bcs::not_a_method does not exist
// CHECK-V2-SUBSTR: closure_bcs::not_a_method not found

// Both raise an invariant violation, whose message is MonoMove's own.
// RUN: execute 0x99::closure_bcs::call_non_persistent_target
// CHECK-V1-SUBSTR: Stored closure references non-persistent function `0x99::closure_bcs::private_add`
// CHECK-V2-SUBSTR: stored closure references non-persistent function `0x99::closure_bcs::private_add`

// Both raise an invariant violation, with different messages.
// RUN: execute 0x99::closure_bcs::call_mask_beyond_params
// CHECK-V1-SUBSTR: Closure signature mismatch for
// CHECK-V2-SUBSTR: mask 0b100 references parameters beyond callee's 2

// RUN: execute 0x99::closure_bcs::call_wrong_type_arg_count
// CHECK-ERROR-PARITY

// Both raise an invariant violation: V1 when `add` reads the `bool`, MonoMove
// when the call decodes it.
// RUN: execute 0x99::closure_bcs::call_capture_layout_mismatch
// CHECK-V1-SUBSTR: INTERNAL_TYPE_ERROR
// CHECK-V2-SUBSTR: capture 0 was stored under a layout the callee's parameter cannot read

// RUN: execute 0x99::closure_bcs::call_unsatisfied_constraint
// CHECK-ERROR-PARITY

// Both raise an invariant violation, with different messages.
// RUN: execute 0x99::closure_bcs::call_param_type_mismatch
// CHECK-V1-SUBSTR: Expected type signer, got address which is not assignable
// CHECK-V2-SUBSTR: closure type does not match the signature of `0x99::closure_bcs::who`

// Both raise an invariant violation, with different messages.
// RUN: execute 0x99::closure_bcs::call_result_count_mismatch
// CHECK-V1-SUBSTR: Closure signature mismatch for
// CHECK-V2-SUBSTR: closure type does not match the signature of `0x99::closure_bcs::add`

// Decoding and filling fit a 288 B heap, so decoding the captures inside the
// call is what collects.
// RUN: execute 0x99::closure_bcs::decode_and_fill --heap-size 288
// CHECK: results: 10
// CHECK-GC-COUNT: 0

// RUN: execute 0x99::closure_bcs::call_with_gc_while_decoding --heap-size 288
// CHECK: results: 13
// CHECK-GC-COUNT: 1
