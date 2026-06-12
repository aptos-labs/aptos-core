// Tests rejection of a (trusted) opaque spec which constrains a closure with
// captured mutations through more than one `ensures_of` condition: the call-site
// model havocs each captured location once and cannot express constraints about
// different applications. (Without `pragma verify = false`, the body check
// refutes such specs already.)
module 0x42::opaque_inline_multi_ensures_fail {

    inline fun call_trusted_twice(f: |u64|) {
        f(1)
    }
    spec call_trusted_twice {
        pragma opaque;
        pragma verify = false;
        ensures ensures_of<f>(1);
        ensures ensures_of<f>(2);
    }

    /// Test: a (trusted) spec constraining one mutating closure through more
    /// than one `ensures_of` cannot be instantiated at the call site.
    fun test_multiple_ensures_of(): u64 {
        let x = 0;
        call_trusted_twice(|i| x = x + i spec { ensures x == old(x) + i; }); // error: more than one ensures_of
        x
    }
    spec test_multiple_ensures_of {
        ensures result == 1;
    }

}
