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
    // The user passes `&mut T` arguments directly (no manual `old(...)`
    // wrapping or explicit post-state args). The model-level rewriter wraps
    // mut-ref inputs in `old(...)` and (for `ensures_of`) appends post-state
    // clones automatically; the Boogie translator emits the canonical
    // pre-state / explicit-result / post-state slots.
    //
    // `result_of<f>` keeps its full return shape: explicit results followed
    // by post-state values of any `&mut` parameters.

    // Test 6: result_of with void function that has mutable ref param
    // f returns () but modifies x, so result_of returns just the modified value (not a tuple)
    fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
    spec apply_void_mut {
        ensures x == result_of<f>(x);
    }

    // Test 7: ensures_of with mutable reference parameter
    fun test_ensures_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec test_ensures_mut {
        ensures ensures_of<f>(x, result);
    }

    // Test 7b: same as Test 7 but with an explicit `old(...)` on the
    // `&mut` arg. The augmenter must still add the post-state slot so
    // the canonical form reaches Boogie with arity 4.
    fun test_ensures_mut_old(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec test_ensures_mut_old {
        ensures ensures_of<f>(old(x), result);
    }

    // Test 8: result_of with mut ref returning a value - using ensures_of to verify
    fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut {
        ensures ensures_of<f>(x, result);
    }

    // Test 9: result_of with function returning value AND modifying &mut param
    // result_of returns (explicit_result, modified_x) tuple, compared via tuple equality
    fun apply_mut_result(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut_result {
        ensures (result, x) == result_of<f>(x);
    }

    // Test 10: result_of tuple with component extraction via let expression
    fun apply_mut_extract(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut_extract {
        ensures result == {let (r, _p) = result_of<f>(x); r};
        ensures x == {let (_r, p) = result_of<f>(x); p};
    }

    // Test 11: result_of with mixed return + &mut, using let to extract and use in expression
    fun apply_mut_arith(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut_arith {
        ensures result + x == {let (r, p) = result_of<f>(x); r + p};
    }

    // Test 12: chained calls to a |&mut u64| closure expressed via state-label
    // composition. `..S |~ ensures_of<f>(x)` says f transforms `old(x)` into
    // the value at S; `S.. |~ ensures_of<f>(x)` says f transforms that value
    // into the final `x`. The intermediate state S is bound by `exists S in *`.
    fun apply_twice(f: |&mut u64| has copy, x: &mut u64) { f(x); f(x) }
    spec apply_twice {
        ensures exists S in *:
            (..S |~ ensures_of<f>(x)) &&
            (S.. |~ ensures_of<f>(x));
    }

}
