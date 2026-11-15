// A callee that uses global memory keeps its behavioral transition even when
// its contract determines every result: substituting the results alone would
// leave a later read of the written memory, and the callee's abort condition,
// stated at the caller's pre-state. This holds for single-result callees
// (`bump1`, `check1`) as it does for multi-result ones.
module 0x42::functional_result_memory {
    struct R has key { v: u64 }

    fun bump1(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        1
    }
    spec bump1 {
        pragma opaque;
        pragma inference = none;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result == 1;
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // The result is 2: the read after the call sees the incremented value.
    fun single(a: address): u64 acquires R {
        let before = borrow_global<R>(a).v;
        let x = bump1(a);
        borrow_global<R>(a).v - before + x
    }

    fun check1(a: address): u64 acquires R {
        let _ = borrow_global<R>(a).v;
        0
    }
    spec check1 {
        pragma opaque;
        pragma inference = none;
        aborts_if !exists<R>(a);
        ensures result == 0;
    }

    // `check1` runs after the publish, so it cannot abort for a missing `R`.
    fun publish_then_check1(s: &signer): u64 acquires R {
        move_to(s, R { v: 0 });
        check1(std::signer::address_of(s))
    }

    fun bump(a: address): (u64, u64) acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        (1, 2)
    }
    spec bump {
        pragma opaque;
        pragma inference = none;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result_1 == 1;
        ensures result_2 == 2;
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // The result is 4: the read after the call sees the incremented value.
    fun multi(a: address): u64 acquires R {
        let before = borrow_global<R>(a).v;
        let (x, y) = bump(a);
        borrow_global<R>(a).v - before + x + y
    }

    fun set2(a: address): (u64, u64) acquires R {
        borrow_global_mut<R>(a).v = 1;
        (0, 0)
    }
    spec set2 {
        pragma opaque;
        pragma inference = none;
        aborts_if !exists<R>(a);
        ensures result_1 == 0;
        ensures result_2 == 0;
        ensures global<R>(a).v == 1;
        modifies global<R>(a);
    }

    // `set2` runs after the publish, so it cannot abort for a missing `R`.
    fun publish_then_set(s: &signer): u64 acquires R {
        move_to(s, R { v: 0 });
        set2(std::signer::address_of(s));
        borrow_global<R>(std::signer::address_of(s)).v
    }

    // A native's effects are its specification's: usage analysis sees no writes.
    native fun native_set2(a: address): (u64, u64);
    spec native_set2 {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures result_1 == 0;
        ensures result_2 == 0;
        ensures exists<R>(a);
        ensures global<R>(a).v == 2;
        modifies global<R>(a);
    }

    // The result is 3 when `R` starts at 1: the read after the call sees the
    // native's write.
    fun read_after_native(a: address): u64 acquires R {
        let before = borrow_global<R>(a).v;
        let (x, y) = native_set2(a);
        borrow_global<R>(a).v + before + x + y
    }

    fun set_v(a: address) acquires R {
        borrow_global_mut<R>(a).v = 3;
    }

    // The write happens in an invoked closure, which bounds no usage summary.
    fun via_closure(a: address): (u64, u64) {
        let f: |address| has drop = |x| set_v(x);
        f(a);
        (0, 0)
    }
    spec via_closure {
        pragma aborts_if_is_partial;
        ensures result_1 == 0;
        ensures result_2 == 0;
    }

    // The result is 4 when `R` starts at 1.
    fun read_after_closure(a: address): u64 acquires R {
        let before = borrow_global<R>(a).v;
        let (x, y) = via_closure(a);
        borrow_global<R>(a).v + before + x + y
    }

    // A callee that only reads memory still has its abort condition at its own
    // call state.
    fun check(a: address): (u64, u64) acquires R {
        let _ = borrow_global<R>(a).v;
        (0, 0)
    }
    spec check {
        pragma opaque;
        pragma inference = none;
        aborts_if !exists<R>(a);
        ensures result_1 == 0;
        ensures result_2 == 0;
    }

    // `check` runs after the publish, so it cannot abort for a missing `R`.
    fun publish_then_check(s: &signer): u64 acquires R {
        move_to(s, R { v: 0 });
        let (x, y) = check(std::signer::address_of(s));
        x + y
    }
}
