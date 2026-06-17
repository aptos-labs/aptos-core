// Tests for inline higher-order functions with `pragma opaque`. Under the prover,
// such inline functions are not expanded at call sites but retained as opaque
// callees: their spec is used at call sites and their body is ignored. Lambda
// arguments are lifted to functions carrying their spec blocks, which back the
// behavioral predicates (`ensures_of` etc.) in the inline function's spec.
module 0x42::opaque_inline_value_capture {

    /// Opaque inline higher-order function: not expanded under the prover,
    /// reasoned about solely via its spec at call sites.
    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// Test: simple lambda without captures.
    fun test_simple(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result == y + 1; }, x)
    }
    spec test_simple {
        ensures result == x + 1;
    }

    /// Test: lambda capturing a context variable by value.
    fun test_value_capture(x: u64, c: u64): u64 {
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec test_value_capture {
        ensures result == x + c;
    }

    /// Test: lambda capturing a local by value.
    fun test_local_capture(x: u64): u64 {
        let c = 7;
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec test_local_capture {
        ensures result == x + 7;
    }

    fun add(x: u64, y: u64): u64 {
        x + y
    }
    spec add {
        ensures result == x + y;
    }

    /// Test: curry-able lambda reducing to a closure over a named function.
    fun test_curry(x: u64): u64 {
        apply(|y| add(3, y), x)
    }
    spec test_curry {
        ensures result == x + 3;
    }

    /// Test: nested calls of the opaque inline function.
    fun test_nested(x: u64): u64 {
        apply(
            |y| y * 2 spec { ensures result == y * 2; },
            apply(|z| z + 5 spec { ensures result == z + 5; }, x)
        )
    }
    spec test_nested {
        ensures result == (x + 5) * 2;
    }

    /// Opaque inline function specified via `result_of`.
    inline fun apply_r(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_r {
        pragma opaque;
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    /// Test: result_of over a value-capturing lambda.
    fun test_result_of(x: u64, c: u64): u64 {
        apply_r(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec test_result_of {
        ensures result == x + c;
    }

    /// Opaque inline function with two function parameters.
    inline fun apply2(f: |u64| u64, g: |u64| u64, x: u64): u64 {
        g(f(x))
    }
    spec apply2 {
        pragma opaque;
        ensures exists m: u64: ensures_of<f>(x, m) && ensures_of<g>(m, result);
    }

    /// Test: two lambdas with distinct value captures in one call.
    fun test_two_lambdas(x: u64, a: u64, b: u64): u64 {
        apply2(
            |y| y + a spec { ensures result == y + a; },
            |y| y + b spec { ensures result == y + b; },
            x
        )
    }
    spec test_two_lambdas {
        requires x < 1000 && a < 1000 && b < 1000;
        ensures result == x + a + b;
    }

    /// Opaque inline function with abort and requires conditions.
    inline fun guarded_apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec guarded_apply {
        pragma opaque;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    /// Test: abort condition of the lambda propagates through the opaque inline call.
    fun test_guarded(x: u64): u64 {
        guarded_apply(
            |y| y + 1 spec {
                aborts_if y + 1 > MAX_U64;
                ensures result == y + 1;
            },
            x
        )
    }
    spec test_guarded {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }
}
