// Expected-failure tests for lambdas capturing mutable references: the
// captured location's post value is constrained exactly by the lambda's
// behavioral spec, no more and no less, and mutations written back into
// global storage are subject to global invariant checking.
module 0x42::opaque_inline_mut_ref_capture_fail {
    struct R has key {
        value: u64,
    }

    spec module {
        // error: global memory invariant does not hold (in test_invariant_violated)
        invariant forall a: address where exists<R>(a): global<R>(a).value < 1000;
    }

    inline fun modify(f: |u64|) {
        f(1)
    }
    spec modify {
        pragma opaque;
        requires !aborts_of<f>(1);
        aborts_if false;
        ensures ensures_of<f>(1);
    }

    /// Test: wrong expectation about the captured location's post value.
    fun test_wrong_post(): u64 {
        let x = 10;
        let r = &mut x;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
        x
    }
    spec test_wrong_post {
        ensures result == 12; // error: post-condition does not hold
    }

    /// Test: the lambda's abort condition must be discharged at the call.
    fun test_abort_not_discharged(x: u64): u64 {
        let r = &mut x;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        }); // error: precondition does not hold (reported at the requires of `modify`)
        x
    }
    spec test_abort_not_discharged {
        ensures result == x + 1;
    }

    /// Test: wrong expectation about the post value of a captured global location.
    fun test_wrong_global_post(a: address) {
        let r = &mut R[a].value;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
    }
    spec test_wrong_global_post {
        requires global<R>(a).value < 999;
        aborts_if !exists<R>(a);
        ensures global<R>(a).value == old(global<R>(a).value) + 2; // error: post-condition does not hold
    }

    /// Test: the module invariant is checked when the captured mutation is
    /// written back into global storage; without a bound on the pre value,
    /// the increment can violate it.
    fun test_invariant_violated(a: address) {
        let r = &mut R[a].value;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
    }
    spec test_invariant_violated {
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).value + 1 > MAX_U64;
    }
}
