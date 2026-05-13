// Tests that assigning to a borrowed non-reference local via a tuple call is rejected.
// Regression test for https://github.com/aptos-labs/aptos-core/issues/19394
module 0xA::M {
    fun foo(): (u64, u64) { (1, 2) }

    // Assigning to `x` which is borrowed via `r` should be rejected.
    fun t1(): u64 {
        let x: u64 = 0;
        let r = &x;
        (x, _) = foo();
        *r
    }
}
