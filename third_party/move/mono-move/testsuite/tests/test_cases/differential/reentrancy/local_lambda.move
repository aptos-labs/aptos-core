// A same-module closure may run while the caller holds a resource borrow,
// provided it performs no global resource access.
// RUN: publish
module 0x42::tests {
    struct R(u64) has key;

    public fun local_lambda_ok(s: signer): u64 acquires R {
        move_to(&s, R(5));
        let r = borrow_global_mut<R>(@0x42);
        let bump = |x: u64| x + 1;
        r.0 = bump(r.0);
        r.0
    }
}

// RUN: execute 0x42::tests::local_lambda_ok --args 0x42
// CHECK: results: 6
