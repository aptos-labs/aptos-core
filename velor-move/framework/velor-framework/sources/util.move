/// Utility functions used by the framework modules.
module velor_framework::util {
    friend velor_framework::code;
    friend velor_framework::gas_schedule;

    /// Native function to deserialize a type T.
    ///
    /// Note that this function does not put any constraint on `T`. If code uses this function to
    /// deserialized a linear value, its their responsibility that the data they deserialize is
    /// owned.
    ///
    /// Function would abort if T has signer in it.
    public(friend) native fun from_bytes<T>(bytes: vector<u8>): T;

    public fun address_from_bytes(bytes: vector<u8>): address {
        from_bytes(bytes)
    }

    #[test_only]
    use std::bcs;

    #[test(s1 = @0x123)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_signer_roundtrip(s1: signer) {
        from_bytes<signer>(bcs::to_bytes(&s1));
    }
}
