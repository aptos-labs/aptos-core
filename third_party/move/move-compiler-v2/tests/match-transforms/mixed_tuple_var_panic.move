/// Test case to check if top-level Var pattern on mixed tuple match
/// triggers a panic in transform_mixed_arm.
module 0xc0ffee::m {
    enum E {
        V(u64),
    }

    fun test(e: E, x: u64): u64 {
        match ((e, x)) {
            t => 0,  // top-level var binding the whole tuple
        }
    }
}
