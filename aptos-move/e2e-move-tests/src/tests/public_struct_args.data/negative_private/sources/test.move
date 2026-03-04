/// Test module with private struct as entry function parameter.
/// This should fail to publish because extended checks reject non-public structs
/// as entry function parameter types.
module 0xcafe::negative_private {

    /// Private struct (not public) - rejected at publish time
    struct PrivatePoint has copy, drop {
        x: u64,
        y: u64,
    }

    /// Entry function that takes private struct - fails at publish time.
    public entry fun test_private(_sender: &signer, _p: PrivatePoint) {
    }
}
