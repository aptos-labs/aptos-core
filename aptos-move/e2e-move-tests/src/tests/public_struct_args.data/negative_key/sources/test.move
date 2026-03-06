/// Test module with a key-ability struct as an entry function parameter.
/// This should fail to publish because structs with the key ability cannot
/// have struct APIs (pack/unpack wrappers) and therefore cannot be used as
/// transaction arguments.
module 0xcafe::negative_key {

    /// Public struct with key ability - rejected at publish time.
    /// Key structs are top-level resources intended for global storage and
    /// are incompatible with being passed as transaction arguments.
    public struct KeyResource has key, copy, drop {
        val: u64,
    }

    /// Fails at publish time: KeyResource has key ability.
    public entry fun test_key_struct(_sender: &signer, _r: KeyResource) {
    }
}
