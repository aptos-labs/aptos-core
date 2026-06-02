// RUN: publish --print(bytecode,stackless,micro-ops)
//
// Global-storage end-to-end parity. The resource is a 32-byte struct with an
// embedded inner struct, so the move_to box (and move_from unbox) copy the
// inline value via the sized HeapMoveTo/HeapMoveFrom path, not the 8-byte fast
// path. The signer and the resource address are passed separately (same value)
// so the test does not depend on the stdlib `signer` module being staged for
// the mono-move path.
module 0x42::globals {
    struct Inner has store, drop { a: u64, b: u64 }
    struct R has key { x: u64, y: u64, inner: Inner }

    // move_to then read every field back via borrow_global.
    public fun publish_and_read(s: signer, a: address, x: u64): u64 {
        move_to(&s, R { x, y: x + 1, inner: Inner { a: x + 2, b: x + 3 } });
        let r = borrow_global<R>(a);
        r.x + r.y + r.inner.a + r.inner.b
    }

    // move_to then move_from, summing the moved-out fields.
    public fun publish_and_take(s: signer, a: address, x: u64): u64 {
        move_to(&s, R { x, y: x + 1, inner: Inner { a: x + 2, b: x + 3 } });
        let R { x: rx, y: ry, inner: Inner { a, b } } = move_from<R>(a);
        rx + ry + a + b
    }

    // borrow_global_mut, mutate a nested field in place, then read it back.
    public fun publish_mutate_read(s: signer, a: address, x: u64): u64 {
        move_to(&s, R { x, y: 0, inner: Inner { a: 0, b: 0 } });
        let r = borrow_global_mut<R>(a);
        r.inner.b = r.x + 1;
        borrow_global<R>(a).inner.b
    }

    // Aborts: publishing the same resource twice.
    public fun double_publish(s: signer) {
        move_to(&s, R { x: 1, y: 1, inner: Inner { a: 1, b: 1 } });
        move_to(&s, R { x: 2, y: 2, inner: Inner { a: 2, b: 2 } });
    }

    // Aborts: borrowing a resource that was never published.
    public fun borrow_missing(a: address): u64 {
        borrow_global<R>(a).x
    }

    // Aborts: moving out a resource that was never published.
    public fun take_missing(a: address): u64 {
        let R { x, y: _, inner: _ } = move_from<R>(a);
        x
    }
}

// RUN: execute 0x42::globals::publish_and_read --args 0x42, 0x42, 100
// CHECK: results: 406

// RUN: execute 0x42::globals::publish_and_take --args 0x7, 0x7, 55
// CHECK: results: 226

// RUN: execute 0x42::globals::publish_mutate_read --args 0x42, 0x42, 41
// CHECK: results: 42

// Abort/missing cases. V1 (MoveVM) surfaces a major VM status while V2
// (mono-move) raises its own RuntimeError, so the two outputs differ — each VM
// is checked with its own substring.
// RUN: execute 0x42::globals::double_publish --args 0x5
// CHECK-V1-SUBSTR: RESOURCE_ALREADY_EXISTS
// CHECK-V2-SUBSTR: already exists

// RUN: execute 0x42::globals::borrow_missing --args 0x123
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobal: resource does not exist

// RUN: execute 0x42::globals::take_missing --args 0x123
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: MoveFrom: resource does not exist
