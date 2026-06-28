// Tests for generic inline higher-order functions with `pragma opaque`,
// and for retained inline-opaque functions without lambda parameters.
module 0x42::opaque_inline_generics {

    /// Retained inline-opaque function without function parameters.
    inline fun add2(x: u64): u64 {
        x + 2
    }
    spec add2 {
        pragma opaque;
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 2;
    }

    fun test_no_lambda(x: u64): u64 {
        add2(x)
    }
    spec test_no_lambda {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 2;
    }

    /// Generic retained inline-opaque higher-order function.
    inline fun apply_gen<T>(f: |T| T, x: T): T {
        f(x)
    }
    spec apply_gen {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun test_generic_instantiated(x: u64, c: u64): u64 {
        apply_gen(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec test_generic_instantiated {
        ensures result == x + c;
    }

    /// Test: capture of a generic-typed value in a generic context.
    fun test_generic_capture<T: copy + drop>(x: T): T {
        apply_gen(|_y| x spec { ensures result == x; }, x)
    }
    spec test_generic_capture {
        ensures result == x;
    }
}
