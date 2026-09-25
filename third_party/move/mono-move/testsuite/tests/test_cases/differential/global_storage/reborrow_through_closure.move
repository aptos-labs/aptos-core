// A closure re-borrows a resource while the caller still holds a `&mut` to
// it. The static acquires check cannot see through the function value, so
// both borrows reach the runtime. V1 rejects the re-borrow with its reentrancy
// check; MonoMove has no such check yet, so both borrows must alias the same
// value.

// RUN: publish
module 0x42::reborrow_through_closure {
    struct R has key {
        count: u64,
    }

    fun bump(a: address) acquires R {
        R[a].count += 1;
    }

    fun apply(f: |address|, a: address) {
        f(a)
    }

    // Control: the first borrow is dead before the second one.
    fun sequential(s: signer, a: address): u64 acquires R {
        move_to(&s, R { count: 0 });
        R[a].count += 1;
        let r = &mut R[a];
        r.count += 1;
        r.count
    }

    fun through_lambda_body(s: signer, a: address): u64 acquires R {
        move_to(&s, R { count: 0 });
        let r = &mut R[a];
        apply(|x| R[x].count += 1, a);
        r.count += 1;
        r.count
    }

    fun through_lambda_call(s: signer, a: address): u64 acquires R {
        move_to(&s, R { count: 0 });
        let r = &mut R[a];
        apply(|x| bump(x), a);
        r.count += 1;
        r.count
    }
}

// RUN: execute 0x42::reborrow_through_closure::sequential --args 0x42, 0x42
// CHECK: results: 2

// RUN: execute 0x42::reborrow_through_closure::through_lambda_body --args 0x42, 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2: results: 2

// RUN: execute 0x42::reborrow_through_closure::through_lambda_call --args 0x42, 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2: results: 2
