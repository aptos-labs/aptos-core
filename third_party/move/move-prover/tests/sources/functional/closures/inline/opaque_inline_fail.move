// Negative tests for inline higher-order functions with `pragma opaque`:
// wrong lambda specs and wrong caller assertions must be reported.
module 0x42::opaque_inline_fail {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// Test: caller postcondition does not follow from the lambda's spec.
    fun test_wrong_caller_post(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result == y + 1; }, x)
    }
    spec test_wrong_caller_post {
        ensures result == x + 2; // error: post-condition does not hold
    }

    /// Test: the lambda's spec does not hold for its body.
    fun test_wrong_lambda_spec(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result == y + 2; }, x) // error: post-condition does not hold (lifted lambda)
    }
    spec test_wrong_lambda_spec {
        ensures result == x + 2;
    }

    /// Test: lambda without a spec block gives no information about the result.
    fun test_no_spec_lambda(x: u64): u64 {
        apply(|y| y + 1, x)
    }
    spec test_no_spec_lambda {
        ensures result == x + 1; // error: post-condition does not hold (no lambda spec to support it)
    }

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// Test: wrong caller postcondition about a modified captured variable.
    fun test_wrong_mut_capture_post(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
    spec test_wrong_mut_capture_post {
        ensures result == 0; // error: post-condition does not hold (x is modified by the lambda)
    }

    /// Test: the lambda's spec about the captured variable does not hold for its body.
    fun test_wrong_mut_capture_lambda_spec(): u64 {
        let x = 0;
        call_once(|i| x = x + i spec { ensures x == old(x) + i + 1; }); // error: post-condition does not hold (lifted lambda)
        x
    }
    spec test_wrong_mut_capture_lambda_spec {
        ensures result == 2;
    }

    struct S has copy, drop {
        x: u64,
        y: u64,
    }

    /// Test: a lambda assigning to a field converts the whole struct variable
    /// into a `&mut` capture; fields not framed by the lambda's spec are havoced.
    fun test_unframed_field(): S {
        let s = S { x: 1, y: 7 };
        call_once(|i| s.x = s.x + i spec { ensures s.x == old(s).x + i; });
        s
    }
    spec test_unframed_field {
        ensures result.x == 2;
        ensures result.y == 7; // error: post-condition does not hold (y is not framed by the lambda spec)
    }

    inline fun update_via(f: |&mut u64|, r: &mut u64) {
        f(r)
    }
    spec update_via {
        pragma opaque;
        ensures ensures_of<f>(r);
    }

    /// Test: post-state slots of the `&mut` parameter and of the modified capture
    /// must not be confused (the claims below are swapped between the two).
    fun test_swapped_mut_posts(p: u64): (u64, u64) {
        let count = 100;
        let v = p;
        update_via(|q| {
            *q = *q + 1;
            count = count + 2;
        } spec {
            ensures q == old(q) + 1;
            ensures count == old(count) + 2;
        }, &mut v);
        (v, count)
    }
    spec test_swapped_mut_posts {
        requires p < 1000;
        // This would verify if the post-state slots were swapped (count's post
        // value is 102); the actual value is `p + 1`.
        ensures result_1 == 102; // error: post-condition does not hold
    }
}
