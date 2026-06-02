// RUN: publish
//
// `exists<R>` parity. A differential test can only populate global storage via
// `move_to` within the same call, so these exercise the working-map presence
// path; the external-storage presence path lives in the runtime crate's tests.
// The signer and the resource address are passed separately (same value) so the
// test does not depend on the stdlib `signer` module being staged for V2.
module 0x42::globals_exists {
    struct R has key { v: u64 }

    // exists on an address with no resource.
    public fun exists_absent(a: address): bool {
        exists<R>(a)
    }

    // move_to then exists.
    public fun exists_after_publish(s: signer, a: address): bool {
        move_to(&s, R { v: 7 });
        exists<R>(a)
    }

    // move_to, move_from, then exists.
    public fun exists_after_take(s: signer, a: address): bool {
        move_to(&s, R { v: 7 });
        let R { v: _ } = move_from<R>(a);
        exists<R>(a)
    }
}

// RUN: execute 0x42::globals_exists::exists_absent --args 0x99
// CHECK: results: false

// RUN: execute 0x42::globals_exists::exists_after_publish --args 0x42, 0x42
// CHECK: results: true

// RUN: execute 0x42::globals_exists::exists_after_take --args 0x7, 0x7
// CHECK: results: false
