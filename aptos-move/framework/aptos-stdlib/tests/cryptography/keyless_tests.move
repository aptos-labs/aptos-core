#[test_only]
module aptos_std::keyless_tests {
    use aptos_std::keyless;
    use std::string::{utf8};
    use std::bcs;

    #[test]
    fun test_deserialize_public_key() {
        // The bytes below represent a Keyless public key that looks like
        // keyless::PublicKey {
        //     iss: "https://accounts.google.com",
        //     idc: "0x86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514"
        // }
        let bytes: vector<u8> = x"1b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514";
        let pk = keyless::new_public_key_from_bytes(bytes);
        assert!(bcs::to_bytes(&pk) == bytes,);
        assert!(pk.get_iss() == utf8(b"https://accounts.google.com"));
        assert!(pk.get_idc() == x"86bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d514");
    }
}
