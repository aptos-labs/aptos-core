/// Test module with private struct as entry function parameter.
/// This should fail because pack functions are only generated for PUBLIC structs.
module 0xcafe::negative_private {

    /// Private struct (not public) - should be rejected as txn arg
    struct PrivatePoint has copy, drop {
        x: u64,
        y: u64,
    }

    /// Entry function that takes private struct - should fail at validation
    /// because no pack function will be generated for PrivatePoint.
    public entry fun test_private(_sender: &signer, _p: PrivatePoint) {
        // This should never execute
    }
}
