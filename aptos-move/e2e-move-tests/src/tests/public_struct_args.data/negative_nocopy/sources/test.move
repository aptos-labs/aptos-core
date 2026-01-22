/// Test module with non-copy struct as entry function parameter.
/// This should fail because pack functions are only generated for public structs WITH copy ability.
module 0xcafe::negative_nocopy {

    /// Public struct without copy ability - should be rejected as txn arg
    public struct NoCopyPoint has drop {
        x: u64,
        y: u64,
    }

    /// Public enum without copy ability - should be rejected as txn arg
    public enum NoCopyColor has drop {
        Red,
        Green,
        Blue,
    }

    /// Entry function that takes non-copy struct - should fail at validation
    /// because no pack function will be generated for NoCopyPoint.
    public entry fun test_no_copy_struct(_sender: &signer, _p: NoCopyPoint) {
        // This should never execute
    }

    /// Entry function that takes non-copy enum - should fail at validation
    /// because no pack function will be generated for NoCopyColor.
    public entry fun test_no_copy_enum(_sender: &signer, _c: NoCopyColor) {
        // This should never execute
    }
}
