#[test_only]
module velor_std::federated_keyless_tests {
    use velor_std::federated_keyless;
    use std::string::{utf8};
    use std::bcs;

    #[test]
    fun test_deserialize_public_key() {
        // The bytes below represent a Federated Keyless public key that looks like
        // federated_keyless::PublicKey {
        //     jwk_address: @0xaa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a,
        //     keyless_public_key: keyless::PublicKey {
        //         iss: "https://accounts.google.com",
        //         idc: "0x86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514"
        //     }
        // }
        //
        let bytes = x"aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a1b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        let pk = federated_keyless::new_public_key_from_bytes(bytes);
        assert!(
            bcs::to_bytes(&pk) == bytes,
        );
        assert!(
            pk.get_keyless_public_key().get_iss() == utf8(b"https://accounts.google.com"),
        );
        assert!(
            pk.get_keyless_public_key().get_idc() == x"86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514",
        );
        assert!(
            pk.get_jwk_address() == @0xaa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a,
        );
    }
}
