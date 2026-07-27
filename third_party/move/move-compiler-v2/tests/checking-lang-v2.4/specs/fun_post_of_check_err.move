// Type checking errors for `fun_post_of<f>(args)`.
module 0x42::fun_post_of_check_err {

    fun apply(f: |u64|u64 has drop, x: u64): u64 {
        f(x)
    }
    spec apply {
        // Error: x is u64, not a function
        ensures fun_post_of<x>(1) == f;
    }

    fun apply2(f: |u64|u64 has drop, x: u64): u64 {
        f(x)
    }
    spec apply2 {
        // Error: wrong number of arguments (inputs only; no result slot)
        ensures fun_post_of<f>(1, 2) == f;
    }

    fun apply3(f: |u64|u64 has drop, x: u64): u64 {
        f(x)
    }
    spec apply3 {
        // Error: result is a function value, not bool
        ensures fun_post_of<f>(1);
    }
}
