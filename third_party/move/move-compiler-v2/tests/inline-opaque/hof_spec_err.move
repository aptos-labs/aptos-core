// Function spec blocks on inline functions with function-typed parameters are
// not supported: the spec would need to refer to the behavior of the lambda
// arguments. This errors in all compilation modes.
module 0x42::hof_spec_err {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        ensures ensures_of<f>(x, result);
    }

    inline fun apply_opaque(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_opaque {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun caller(x: u64): u64 {
        apply(|y| y + 1, x) + apply_opaque(|y| y + 1, x)
    }
}
