// RUN: publish --print(micro-ops)
module 0x42::generic_global_in_generic_fn {
    struct S<T> has key {
        value: T,
    }

    struct D<T> has key, drop {
        value: T,
    }

    fun put<T: store>(s: &signer, v: T) {
        move_to(s, S<T> { value: v });
    }

    fun take<T: store>(a: address): T {
        let S { value } = move_from<S<T>>(a);
        value
    }

    // `exists<S<T>>` with no Pack/Unpack of `S<T>` in the body.
    fun is_present<T: store>(a: address): bool {
        exists<S<T>>(a)
    }

    // `borrow_global<S<T>>` (immutable) with no Pack of `S<T>`.
    fun refs_equal<T: store>(a: address): bool {
        borrow_global<S<T>>(a) == borrow_global<S<T>>(a)
    }

    fun put_d<T: store + drop>(s: &signer, v: T) {
        move_to(s, D<T> { value: v });
    }

    fun bump_d<T: store + drop>(a: address, v: T) {
        *borrow_global_mut<D<T>>(a) = D<T> { value: v };
    }

    fun take_d<T: store + drop>(a: address): T {
        let D { value } = move_from<D<T>>(a);
        value
    }

    fun roundtrip_u64(s: signer, a: address, v: u64): u64 {
        put<u64>(&s, v);
        take<u64>(a)
    }

    fun roundtrip_bool(s: signer, a: address, b: bool): u64 {
        put<bool>(&s, b);
        if (take<bool>(a)) { 1 } else { 0 }
    }

    // exists<S<u64>> before (present) and after (absent) move_from.
    fun presence_u64(s: signer, a: address, v: u64): u64 {
        put<u64>(&s, v);
        let p1 = if (is_present<u64>(a)) { 10 } else { 0 };
        let _ = take<u64>(a);
        let p2 = if (is_present<u64>(a)) { 1 } else { 0 };
        p1 + p2
    }

    // borrow_global<S<u64>> twice and compare the referents.
    fun equal_u64(s: signer, a: address, v: u64): u64 {
        put<u64>(&s, v);
        let e = if (refs_equal<u64>(a)) { 1 } else { 0 };
        let _ = take<u64>(a);
        e
    }

    fun mutate_u64(s: signer, a: address, v: u64): u64 {
        put_d<u64>(&s, v);
        bump_d<u64>(a, v + 5);
        take_d<u64>(a)
    }

    fun both(s: signer, a: address, v: u64): u64 {
        put<u64>(&s, v);
        put<bool>(&s, true);
        let present = if (is_present<u64>(a) && is_present<bool>(a)) { 100 } else { 0 };
        let x = take<u64>(a);
        let flag = take<bool>(a);
        let gone = if (!is_present<u64>(a) && !is_present<bool>(a)) { 1 } else { 0 };
        x + present + (if (flag) { 10 } else { 0 }) + gone
    }

    // Aborts: double-publish the same instantiation.
    fun double_publish_u64(s: signer, v: u64) {
        put<u64>(&s, v);
        put<u64>(&s, v);
    }

    // Aborts: move out an instantiation that was never published.
    fun take_missing_bool(a: address): u64 {
        if (take<bool>(a)) { 1 } else { 0 }
    }
}

// RUN: execute 0x42::generic_global_in_generic_fn::roundtrip_u64 --args 0x55, 0x55, 7
// CHECK: results: 7

// RUN: execute 0x42::generic_global_in_generic_fn::roundtrip_bool --args 0x56, 0x56, true
// CHECK: results: 1

// RUN: execute 0x42::generic_global_in_generic_fn::presence_u64 --args 0x57, 0x57, 3
// CHECK: results: 10

// RUN: execute 0x42::generic_global_in_generic_fn::equal_u64 --args 0x58, 0x58, 9
// CHECK: results: 1

// RUN: execute 0x42::generic_global_in_generic_fn::mutate_u64 --args 0x59, 0x59, 10
// CHECK: results: 15

// RUN: execute 0x42::generic_global_in_generic_fn::both --args 0x60, 0x60, 50
// CHECK: results: 161

// Abort cases: V1 surfaces a major VM status, V2 raises its own RuntimeError,
// so each VM is checked with its own substring.
// RUN: execute 0x42::generic_global_in_generic_fn::double_publish_u64 --args 0x61, 4
// CHECK-V1-SUBSTR: RESOURCE_ALREADY_EXISTS
// CHECK-V2-SUBSTR: already exists

// RUN: execute 0x42::generic_global_in_generic_fn::take_missing_bool --args 0x62
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: MoveFrom: resource does not exist
