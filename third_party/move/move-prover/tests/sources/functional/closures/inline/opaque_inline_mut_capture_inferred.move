// Test for a mut-capturing lambda without an explicit spec block: the lambda's
// spec (relating the captured location's pre and post state) is inferred from
// its body by lambda spec inference.
module 0x42::opaque_inline_mut_capture_inferred {
    inline fun modify(f: |u64|) {
        f(1)
    }
    spec modify {
        pragma opaque;
        requires !aborts_of<f>(1);
        aborts_if false;
        ensures ensures_of<f>(1);
    }

    /// Test: the lambda's relational spec is inferred.
    fun test_inferred_lambda_spec(): u64 {
        let x = 10;
        let r = &mut x;
        modify(|y| *r = *r + y);
        x
    }
    spec test_inferred_lambda_spec {
        ensures result == 11;
    }
}
