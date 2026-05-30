// Test cases for result_of behavioral predicate
// result_of<f>(x) returns a deterministic result based on ensures_of<f>(x, y)
// Semantics: result_of<f>(x) == choose y where ensures_of<f>(x, y)
module 0x42::behavioral_results {

    // Test 1: Basic result_of with simple function
    fun apply(f: |u64| u64, x: u64): u64 { f(x) }
    spec apply {
        ensures result == result_of<f>(x);
    }

    // Test 2: result_of with known function
    fun double(x: u64): u64 { x * 2 }
    spec double { ensures result == x * 2; }

    fun test_known(): u64 { double(5) }
    spec test_known {
        ensures result == result_of<double>(5);
    }

    // Test 3: result_of in sequential application
    fun apply_seq(f: |u64| u64 has copy, x: u64): u64 { f(f(x)) }
    spec apply_seq {
        // First application
        let y = result_of<f>(x);
        // Second application uses result of first
        ensures result == result_of<f>(y);
    }

    // Test 4: result_of with multiple parameters
    fun apply2(f: |u64, u64| u64, x: u64, y: u64): u64 { f(x, y) }
    spec apply2 {
        ensures result == result_of<f>(x, y);
    }

    // Test 5: result_of with known function taking multiple parameters
    fun add(x: u64, y: u64): u64 { x + y }
    spec add { ensures result == x + y; }

    fun test_add(): u64 { add(3, 4) }
    spec test_add {
        ensures result == result_of<add>(3, 4);
    }

    // ===== Tests for mutable reference parameters =====
    //
    // `&mut T` arguments appear exactly once in a behavioral-predicate call;
    // the Boogie backend handles the in/out split by emitting pre-state into
    // the input slot and the live (post-state) form into the matching
    // `ensures_of` post slot. `result_of<f>` returns only `f`'s declared
    // return type and is only valid on non-void callees; for the post-state
    // of a `&mut` argument, use `ensures_of<f>(x, result)` as a relation.

    // Test 6: void callee with `&mut` param — `result_of` is invalid here,
    // so use `ensures_of` directly to relate pre-/post-state.
    fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
    spec apply_void_mut {
        ensures ensures_of<f>(x);
    }

    // Test 7: ensures_of with mutable reference parameter
    fun test_ensures_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec test_ensures_mut {
        ensures ensures_of<f>(x, result);
    }

    // Test 8: result_of returning the declared scalar return of a `|&mut T| R`
    // closure — `x` appears only once on the right-hand side, as the
    // pre-state input.
    fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut {
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // Test 9: symmetric form combining a scalar `result_of` postcondition
    // with `ensures_of` for the `&mut` post-state constraint.
    fun apply_mut_sym(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut_sym {
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // Test 10: chained calls to a `|&mut u64|` closure expressed via
    // state-label composition. `..S |~ ensures_of<f>(x)` says `f` transforms
    // `old(x)` into the value at `S`; `S.. |~ ensures_of<f>(x)` says `f`
    // transforms that value into the final `x`. The intermediate state `S`
    // is bound by `exists S in *`.
    fun apply_twice(f: |&mut u64| has copy, x: &mut u64) { f(x); f(x) }
    spec apply_twice {
        ensures exists S in *:
            (..S |~ ensures_of<f>(x)) &&
            (S.. |~ ensures_of<f>(x));
    }

    // Test 11: canonical-form `ensures_of` with an explicit post-state slot.
    // Equivalent to Test 9 — both express the same pre/post relation; the
    // parser accepts either the minimum form `ensures_of<f>(x, r)` (Test 9)
    // or the canonical form `ensures_of<f>(x_pre, r, x_post)` (this test).
    fun apply_mut_canonical(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut_canonical {
        ensures result == result_of<f>(x);
        // Three args after `f`: pre-state of `x`, the result, post-state of `x`.
        ensures ensures_of<f>(old(x), result, x);
    }

    // Test 12: two `&mut` parameters. `result_of<f>(p, q)` returns the
    // declared `u64` return; `ensures_of<f>(p, q, result)` constrains the
    // pre/post relation for both `&mut` slots simultaneously.
    fun apply_two_mut(f: |&mut u64, &mut u64| u64, p: &mut u64, q: &mut u64): u64 {
        f(p, q)
    }
    spec apply_two_mut {
        ensures result == result_of<f>(p, q);
        ensures ensures_of<f>(p, q, result);
    }

    // Test 13: multi-return closure plus `&mut`. `result_of<f>(x)` returns
    // the declared 2-tuple `(u64, u64)`; the Boogie backend extracts the
    // declared-results sub-tuple from the extended Skolem (which also
    // carries the `&mut` post-state). The spec compares the procedure's
    // two return values to the projected components.
    fun apply_mut_multi(f: |&mut u64| (u64, u64), x: &mut u64): (u64, u64) { f(x) }
    spec apply_mut_multi {
        ensures (result_1, result_2) == result_of<f>(x);
    }

    // Test 14: labeled subexpression inside a quantifier body that
    // references the bound variable. Clause-level CSE must not hoist
    // `..S |~ result_of<g>(i)` outside the quantifier — `i` would no
    // longer be in scope at the let-binding site.
    fun query_at(g: |u64| u64, n: u64): bool {
        let _ = g(0);
        n > 0
    }
    spec query_at {
        ensures result == (n > 0);
        ensures forall i: u64 where i < n:
            result_of<g>(i) >= 0;
    }

}
