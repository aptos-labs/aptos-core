#[test_only]
module aptos_std::single_key_tests {
    use aptos_std::single_key;

    #[test]
    #[expected_failure(abort_code = 0x10002, location = single_key)]
    fun test_deserialize_fails_for_extra_bytes() {
        let pk_bytes: vector<u8> = x"031b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        pk_bytes.push_back(0x01);
        let _any_pk = single_key::new_public_key_from_bytes(pk_bytes);
    }

    #[test]
    fun test_get_authentication_key() {
        let pk_bytes: vector<u8> = x"031b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        let any_pk = single_key::new_public_key_from_bytes(pk_bytes);
        assert!(
            any_pk.to_authentication_key() == x"69d542afebf0387b5e4fcb447b79e3fa9b9aaadba4697b51b90b8d7b9649d159",
            std::error::invalid_state(1)
        );
    }
}
