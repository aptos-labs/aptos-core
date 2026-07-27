// Tests `fun_post_of` chains over `&mut`-capturing closures at retained
// inline-opaque calls, and the opt-in two-state reading of fun params: when a
// spec mentions `old(f)` or `fun_post_of`, bare `f` in post conditions
// denotes the final value and `old(f)` the value at entry (like a `&mut`
// param). The chain `fun_post_of<fun_post_of<old(f)>(1)>(2)` describes
// exactly two applications in order: the callee can prove it only with a
// conforming body, and the caller unfolds it through the closure's spec.
module 0x42::opaque_inline_fun_post_of {

    inline fun call_once(f: |u64| has drop) {
        f(2)
    }
    spec call_once {
        pragma opaque;
        ensures f == fun_post_of<old(f)>(2);
    }

    /// Test: a single application described by `fun_post_of` instead of
    /// `ensures_of`.
    fun test_single(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_single {
        ensures result == 2;
    }

    inline fun call_twice(f: |u64| has copy + drop) {
        f(1);
        f(2);
    }
    spec call_twice {
        pragma opaque;
        ensures f == fun_post_of<fun_post_of<old(f)>(1)>(2);
    }

    /// Test: chained applications accumulate both effects at the caller.
    fun test_chain(): u64 {
        let x = 0;
        call_twice(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_chain {
        ensures result == 3;
    }

    inline fun apply_pure(f: |u64|u64 has drop, x: u64): u64 {
        f(x)
    }
    spec apply_pure {
        pragma opaque;
        ensures result == result_of<f>(x);
        ensures f == fun_post_of<old(f)>(x);
    }

    /// Test: for values which cannot carry mutations, `fun_post_of` is the
    /// identity, so the chain claim holds trivially.
    fun test_identity(y: u64): u64 {
        apply_pure(|v| v + 1, y)
    }
    spec test_identity {
        requires y < 1000;
        ensures result == y + 1;
    }
}
