/// Test module with non-copy struct as entry function parameter.
/// This should fail to publish because extended checks require entry function parameter types
/// to be public structs WITH copy ability.
module 0xcafe::negative_nocopy {

    /// Public struct without copy ability - rejected at publish time
    public struct NoCopyPoint has drop {
        x: u64,
        y: u64,
    }

    /// Public enum without copy ability - rejected at publish time
    public enum NoCopyColor has drop {
        Red,
        Green,
        Blue,
    }

    /// Fails at publish time: NoCopyPoint lacks copy ability.
    public entry fun test_no_copy_struct(_sender: &signer, _p: NoCopyPoint) {
    }

    /// Fails at publish time: NoCopyColor lacks copy ability.
    public entry fun test_no_copy_enum(_sender: &signer, _c: NoCopyColor) {
    }
}
