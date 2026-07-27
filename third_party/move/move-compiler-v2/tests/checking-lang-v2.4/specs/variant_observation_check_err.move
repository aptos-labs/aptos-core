// Type checking errors for `partial_of` / `captures_of` / `write_of`.
module 0x42::variant_observation_check_err {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        ensures s == old(s) + e;
    }

    fun no_capture(e: &u64): u64 {
        *e
    }

    fun apply1(f: |&u64| has copy + drop, x: u64): u64 {
        f(&x);
        x
    }
    spec apply1 {
        // Error: x is u64, not a function
        ensures partial_of<x>(step);
    }

    fun apply2(f: |&u64| has copy + drop, x: u64): u64 {
        f(&x);
        x
    }
    spec apply2 {
        // Error: argument must be a simple function name
        ensures partial_of<f>(x + 1);
    }

    fun apply3(f: |&u64| has copy + drop, x: u64): u64 {
        f(&x);
        x
    }
    spec apply3 {
        // Error: target has no leading `&mut` (captured) parameter
        ensures captures_of<f>(no_capture) == 0;
    }

    fun apply4(f: |&u64| has copy + drop, x: u64): u64 {
        f(&x);
        x
    }
    spec apply4 {
        // Error: `write_of` index exceeds the number of `&mut` parameters
        ensures write_of<step, 3>(1, 2) == 0;
    }

    fun apply5(f: |&u64| has copy + drop, x: u64): u64 {
        f(&x);
        x
    }
    spec apply5 {
        // Error: captured value is u64, not bool
        ensures captures_of<f>(step);
    }
}
