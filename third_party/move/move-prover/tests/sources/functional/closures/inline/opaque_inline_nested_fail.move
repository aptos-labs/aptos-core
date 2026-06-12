// Tests that a lambda which itself contains a retained inline-opaque call with
// a mutating capture is rejected: inside the lifted outer lambda, the inner
// closure captures the outer's `&mut` parameter rather than a direct borrow of
// a local, which the call-site model does not support.
module 0x42::opaque_inline_nested_fail {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    inline fun call_outer(g: ||) {
        g()
    }
    spec call_outer {
        pragma opaque;
        ensures ensures_of<g>();
    }

    fun test_nested_mut_capture(): u64 {
        let x = 0;
        call_outer(|| call_once(|i| x = x + i spec { ensures x == old(x) + i; }) // error: captured mutable reference must be a direct borrow
            spec { ensures x == old(x) + 1; });
        x
    }
    spec test_nested_mut_capture {
        ensures result == 1;
    }
}
