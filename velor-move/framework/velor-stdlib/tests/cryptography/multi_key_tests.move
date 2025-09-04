#[test_only]
module velor_std::multi_key_tests {
    use velor_std::single_key;
    use velor_std::multi_key;
    use std::bcs;
    #[test]
    fun test_construct_multi_key() {
        let pk1 = single_key::new_public_key_from_bytes(x"0020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a");
        let pk2 = single_key::new_public_key_from_bytes(x"0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e");
        let multi_key = multi_key::new_multi_key_from_single_keys(vector[pk1, pk2], 1);
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        assert!(bcs::to_bytes(&multi_key) == mk_bytes);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = multi_key)]
    fun test_construct_multi_key_bad_input_signatures_required_too_large() {
        let pk1 = single_key::new_public_key_from_bytes(x"0020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a");
        let pk2 = single_key::new_public_key_from_bytes(x"0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e");
        let _multi_key = multi_key::new_multi_key_from_single_keys(vector[pk1, pk2], 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = multi_key)]
    fun test_construct_multi_key_bad_input_no_keys() {
        let _multi_key = multi_key::new_multi_key_from_single_keys(vector[], 1);
    }

    #[test]
    fun test_construct_multi_key_from_bytes() {
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        let multi_key = multi_key::new_public_key_from_bytes(mk_bytes);
        assert!(bcs::to_bytes(&multi_key) == mk_bytes, std::error::invalid_state(1));
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = multi_key)]
    fun test_construct_multi_key_from_bytes_bad_input_extra_bytes() {
        let mk_bytes: vector<u8> = x"020020aa9b5e7acc48169fdc3809b614532a5a675cf7d4c80cd4aea732b47e328bda1a0020bd182d6e3f4ad1daf0d94e53daaece63ebd571d8a8e0098a02a4c0b4ecc7c99e01";
        mk_bytes.push_back(0x01);
        let _multi_key = multi_key::new_public_key_from_bytes(mk_bytes);
    }

    #[test]
    fun test_get_authentication_key() {
        let mk_bytes: vector<u8> = x"02031b68747470733a2f2f6163636f756e74732e676f6f676c652e636f6d2086bc0a0a825eb6337ca1e8a3157e490eac8df23d5cef25d9641ad5e7edc1d51400205da515f392de68080051559c9d9898f5feb377f0b0f15d43fd01c98f0a63b0d801";
        let multi_key = multi_key::new_public_key_from_bytes(mk_bytes);
        assert!(
            multi_key.to_authentication_key() == x"c7ab91daf558b00b1f81207b702349a74029dddfbf0e99d54b3d7675714a61de",
        );
    }
}
