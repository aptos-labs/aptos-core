// Tests for lambdas modifying captured variables at retained inline-opaque
// call sites in various control-flow and shape contexts.
module 0x42::opaque_inline_mut_capture_control {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// Test: calls under different branches.
    fun test_branches(b: bool): u64 {
        let x = 0;
        if (b) {
            call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        } else {
            call_once(|i| x = x + 2 * i spec { ensures x == old(x) + 2 * i; });
        };
        x
    }
    spec test_branches {
        ensures b ==> result == 1;
        ensures !b ==> result == 2;
    }

    /// Test: call in a loop with an invariant over the captured variable.
    fun test_loop(n: u64): u64 {
        let x = 0;
        let k = 0;
        while (k < n) {
            call_once(|i| x = x + i spec { ensures x == old(x) + i; });
            k = k + 1;
        } spec {
            invariant x == k;
            invariant k <= n;
        };
        x
    }
    spec test_loop {
        ensures result == n;
    }

    /// Test: an inner let shadowing the captured name is not confused with it.
    fun test_shadowing(): u64 {
        let x = 1;
        call_once(|i| {
            x = x + i;
            let x = 100;
            assert!(x == 100, 1);
        } spec {
            ensures x == old(x) + i;
        });
        x
    }
    spec test_shadowing {
        ensures result == 2;
    }

    /// Test: explicitly created `&mut` to a local, captured by the lambda.
    fun test_explicit_ref_capture(): u64 {
        let x = 1;
        let r = &mut x;
        call_once(|i| *r = *r + i spec { ensures r == old(r) + i; });
        x
    }
    spec test_explicit_ref_capture {
        ensures result == 2;
    }

    /// Test: lambda with multiple parameters and a modified capture.
    inline fun call2(f: |u64, u64|) {
        f(1, 2)
    }
    spec call2 {
        pragma opaque;
        ensures ensures_of<f>(1, 2);
    }
    fun test_two_lambda_params(): u64 {
        let x = 0;
        call2(|a, b| x = x + a + b spec { ensures x == old(x) + a + b; });
        x
    }
    spec test_two_lambda_params {
        ensures result == 3;
    }

    /// Test: two lambdas mutating distinct captured variables in one call;
    /// the callee invokes them sequentially.
    inline fun call_both(f: |u64|, g: |u64|) {
        f(1);
        g(1)
    }
    spec call_both {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures ensures_of<g>(1);
    }
    fun test_two_mut_lambdas(): u64 {
        let x = 0;
        let y = 10;
        call_both(
            |i| x = x + i spec { ensures x == old(x) + i; },
            |i| y = y + 2 * i spec { ensures y == old(y) + 2 * i; }
        );
        x + y
    }
    spec test_two_mut_lambdas {
        ensures result == 13;
    }

    /// Test: a direct non-reference argument of a mutated local is allowed; it
    /// is evaluated at call entry under both closure and expansion semantics.
    inline fun call_with(f: |u64|, v: u64): u64 {
        f(1);
        v
    }
    spec call_with {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures result == v;
    }
    fun test_mut_capture_and_value_arg(): (u64, u64) {
        let x = 1;
        let r = call_with(|i| x = x + i spec { ensures x == old(x) + i; }, x);
        (x, r)
    }
    spec test_mut_capture_and_value_arg {
        ensures result_1 == 2;
        ensures result_2 == 1;
    }

    /// Test: abort condition of a lambda which modifies a capture propagates
    /// through the opaque call.
    inline fun guarded_once(f: |u64|) {
        f(1)
    }
    spec guarded_once {
        pragma opaque;
        aborts_if aborts_of<f>(1);
        ensures ensures_of<f>(1);
    }
    fun test_guarded_mut_capture(): u64 {
        let x = 10;
        guarded_once(|i| x = x + i spec {
            aborts_if x + i > MAX_U64;
            ensures x == old(x) + i;
        });
        x
    }
    spec test_guarded_mut_capture {
        aborts_if false;
        ensures result == 11;
    }

    /// Test: value-returning lambda which also modifies a capture; the declared
    /// result and the capture's post-state are both constrained.
    inline fun apply_once(f: |u64| u64): u64 {
        f(1)
    }
    spec apply_once {
        pragma opaque;
        ensures result == result_of<f>(1);
        ensures ensures_of<f>(1, result);
    }
    fun test_result_and_mut_capture(): u64 {
        let x = 5;
        let r = apply_once(|i| {
            x = x + i;
            x
        } spec {
            ensures x == old(x) + i;
            ensures result == old(x) + i;
        });
        x + r
    }
    spec test_result_and_mut_capture {
        ensures result == 12;
    }

    /// Test: the callee spec may alias the function value through a spec `let`.
    inline fun call_aliased(f: |u64|) {
        f(1)
    }
    spec call_aliased {
        pragma opaque;
        let g = f;
        ensures ensures_of<g>(1);
    }
    fun test_spec_let_alias(): u64 {
        let x = 0;
        call_aliased(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_spec_let_alias {
        ensures result == 1;
    }

    spec module {
        global sum_ghost: u64;
    }

    /// Test: a ghost update over `result_of` of a mutating lambda is rewritten
    /// like the other spec conditions and constrains the ghost variable.
    inline fun apply_tracked(f: |u64| u64): u64 {
        f(1)
    }
    spec apply_tracked {
        pragma opaque;
        ensures result == result_of<f>(1);
        ensures ensures_of<f>(1, result);
        update sum_ghost = result_of<f>(1);
    }
    fun test_ghost_update(): u64 {
        let x = 5;
        let r = apply_tracked(|i| {
            x = x + i;
            x
        } spec {
            ensures x == old(x) + i;
            ensures result == old(x) + i;
        });
        spec {
            assert sum_ghost == 6;
        };
        x + r
    }
    spec test_ghost_update {
        ensures result == 12;
    }

    /// Test: nested field mutation of a captured struct.
    struct Inner has copy, drop {
        v: u64,
    }
    struct Outer has copy, drop {
        i: Inner,
        w: u64,
    }
    fun test_nested_field(): Outer {
        let o = Outer { i: Inner { v: 1 }, w: 9 };
        call_once(|k| o.i.v = o.i.v + k spec {
            ensures o.i.v == old(o).i.v + k;
            ensures o.w == old(o).w;
        });
        o
    }
    spec test_nested_field {
        ensures result.i.v == 2;
        ensures result.w == 9;
    }
}
