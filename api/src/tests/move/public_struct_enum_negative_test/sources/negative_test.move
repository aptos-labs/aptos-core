/// Module with invalid struct/enum types for negative testing
module account::negative_test {

    /// Private struct (not public) - should be rejected as txn arg
    struct PrivatePoint has copy, drop {
        x: u64,
        y: u64,
    }

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

    /// Entry function that takes private struct - should fail at validation
    public entry fun test_private_struct(_sender: &signer, _p: PrivatePoint) {
        // This should never execute
    }

    /// Entry function that takes non-copy struct - should fail at validation
    public entry fun test_no_copy_struct(_sender: &signer, _p: NoCopyPoint) {
        // This should never execute
    }

    /// Entry function that takes non-copy enum - should fail at validation
    public entry fun test_no_copy_enum(_sender: &signer, _c: NoCopyColor) {
        // This should never execute
    }
}
