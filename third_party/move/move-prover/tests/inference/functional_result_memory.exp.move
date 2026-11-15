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
    spec single(a: address): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        modifies R[a];
        ensures [inferred] result == R[a].v - old(R[a]).v + result_of<bump1>(a);
        ensures [inferred] ensures_of<bump1>(a, result_of<bump1>(a));
        aborts_if [inferred] aborts_of<bump1>(a);
        aborts_if [inferred] !exists<R>(a);
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
    spec publish_then_check1(s: &signer): u64 {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] result == (S1.. |~ result_of<check1>(signer::address_of(s)));
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ publish<R>(a, R{v: 0})
        };
        aborts_if [inferred] exists<R>(signer::address_of(s));
        aborts_if [inferred] S1 |~ (aborts_of<check1>(signer::address_of(s)));
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
    spec multi(a: address): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        modifies R[a];
        ensures [inferred] ({
            let (a_0,a_1) = result_of<bump>(a);
            result == R[a].v - old(R[a]).v + a_0 + a_1
        });
        ensures [inferred] ({
            let (a_0,a_1) = result_of<bump>(a);
            ensures_of<bump>(a, a_0, a_1)
        });
        aborts_if [inferred] aborts_of<bump>(a);
        aborts_if [inferred] !exists<R>(a);
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
    spec publish_then_set(s: &signer): u64 {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] result == R[signer::address_of(s)].v;
        ensures [inferred] ({
            let (a_0,a_1) = S1.. |~ result_of<set2>(signer::address_of(s));
            S1.. |~ ensures_of<set2>(signer::address_of(s), a_0, a_1)
        });
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ publish<R>(a, R{v: 0})
        };
        aborts_if [inferred] exists<R>(signer::address_of(s));
        aborts_if [inferred] S1 |~ (aborts_of<set2>(signer::address_of(s)));
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
    spec read_after_native(a: address): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        modifies R[a];
        ensures [inferred] ({
            let (a_0,a_1) = result_of<native_set2>(a);
            result == R[a].v + old(R[a]).v + a_0 + a_1
        });
        ensures [inferred] ({
            let (a_0,a_1) = result_of<native_set2>(a);
            ensures_of<native_set2>(a, a_0, a_1)
        });
        aborts_if [inferred] aborts_of<native_set2>(a);
        aborts_if [inferred] !exists<R>(a);
    }


    fun set_v(a: address) acquires R {
        borrow_global_mut<R>(a).v = 3;
    }
    spec set_v(a: address) {
        pragma opaque = true;
        modifies R[a];
        ensures [inferred] update<R>(a, update_field(old(R[a]), v, 3));
        aborts_if [inferred] !exists<R>(a);
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
        pragma opaque = true;
        ensures [inferred] result_1 == 0;
        ensures [inferred] result_2 == 0;
        ensures [inferred] ensures_of<set_v>(a);
        aborts_if [inferred] aborts_of<set_v>(a);
    }

    // The result is 4 when `R` starts at 1.
    fun read_after_closure(a: address): u64 acquires R {
        let before = borrow_global<R>(a).v;
        let (x, y) = via_closure(a);
        borrow_global<R>(a).v + before + x + y
    }
    spec read_after_closure(a: address): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] ({
            let (a_0,a_1) = result_of<via_closure>(a);
            result == R[a].v + old(R[a]).v + a_0 + a_1
        });
        ensures [inferred] ({
            let (a_0,a_1) = result_of<via_closure>(a);
            ensures_of<via_closure>(a, a_0, a_1)
        });
        aborts_if [inferred] !exists<R>(a);
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
    spec publish_then_check(s: &signer): u64 {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] ({
            let (a_0,a_1) = S1.. |~ result_of<check>(signer::address_of(s));
            result == a_0 + a_1
        });
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ publish<R>(a, R{v: 0})
        };
        aborts_if [inferred] exists<R>(signer::address_of(s));
        aborts_if [inferred] S1 |~ (aborts_of<check>(signer::address_of(s)));
        aborts_if [inferred] ({
            let (a_0,a_1) = S1.. |~ result_of<check>(signer::address_of(s));
            a_0 + a_1 > MAX_U64
        });
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `functional_result_memory::single` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
   ┌─ tests/inference/functional_result_memory.move:25:5
   │
25 │ ╭     fun single(a: address): u64 acquires R {
26 │ │         let before = borrow_global<R>(a).v;
27 │ │         let x = bump1(a);
28 │ │         borrow_global<R>(a).v - before + x
29 │ │     }
   │ ╰─────^

warning: WP could not characterize the aborts of `functional_result_memory::multi` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
   ┌─ tests/inference/functional_result_memory.move:65:5
   │
65 │ ╭     fun multi(a: address): u64 acquires R {
66 │ │         let before = borrow_global<R>(a).v;
67 │ │         let (x, y) = bump(a);
68 │ │         borrow_global<R>(a).v - before + x + y
69 │ │     }
   │ ╰─────^

warning: WP could not characterize the aborts of `functional_result_memory::publish_then_set` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
   ┌─ tests/inference/functional_result_memory.move:86:5
   │
86 │ ╭     fun publish_then_set(s: &signer): u64 acquires R {
87 │ │         move_to(s, R { v: 0 });
88 │ │         set2(std::signer::address_of(s));
89 │ │         borrow_global<R>(std::signer::address_of(s)).v
90 │ │     }
   │ ╰─────^

warning: WP could not characterize the aborts of `functional_result_memory::read_after_native` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
    ┌─ tests/inference/functional_result_memory.move:106:5
    │
106 │ ╭     fun read_after_native(a: address): u64 acquires R {
107 │ │         let before = borrow_global<R>(a).v;
108 │ │         let (x, y) = native_set2(a);
109 │ │         borrow_global<R>(a).v + before + x + y
110 │ │     }
    │ ╰─────^

warning: WP could not characterize the aborts of `functional_result_memory::read_after_closure` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::functional_result_memory::via_closure` has no trusted complete abort summary
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
    ┌─ tests/inference/functional_result_memory.move:129:5
    │
129 │ ╭     fun read_after_closure(a: address): u64 acquires R {
130 │ │         let before = borrow_global<R>(a).v;
131 │ │         let (x, y) = via_closure(a);
132 │ │         borrow_global<R>(a).v + before + x + y
133 │ │     }
    │ ╰─────^

Verification: Succeeded.
*/
