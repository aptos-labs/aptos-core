// RUN: publish
module 0x42::globals_fn {
    struct R has key { v: u64 }

    // borrow_global_mut on a missing resource aborts.
    public fun borrow_mut_missing(a: address): u64 {
        borrow_global_mut<R>(a).v
    }

    // move_to, move_from, then borrow_global — the resource is gone, so this
    // aborts.
    public fun take_then_borrow(s: signer, a: address): u64 {
        move_to(&s, R { v: 9 });
        let R { v: _ } = move_from<R>(a);
        borrow_global<R>(a).v
    }

    // Publish distinct values at two addresses, then read both back; the result
    // encodes both values, proving the keys do not alias.
    public fun two_keys(s1: signer, s2: signer, a1: address, a2: address): u64 {
        move_to(&s1, R { v: 100 });
        move_to(&s2, R { v: 5 });
        borrow_global<R>(a1).v * 1000 + borrow_global<R>(a2).v
    }
}

// RUN: execute 0x42::globals_fn::borrow_mut_missing --args 0x123
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobalMut: resource does not exist

// RUN: execute 0x42::globals_fn::take_then_borrow --args 0x7, 0x7
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobal: resource does not exist

// RUN: execute 0x42::globals_fn::two_keys --args 0x1, 0x2, 0x1, 0x2
// CHECK: results: 100005
