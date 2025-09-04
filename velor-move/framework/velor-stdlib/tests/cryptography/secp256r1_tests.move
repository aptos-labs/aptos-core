#[test_only]
module velor_std::secp256r1_tests {
    use velor_std::secp256r1;

    #[test]
    #[expected_failure(abort_code = 0x10001, location = secp256r1)]
    fun test_ecdsa_raw_public_key_from_64_bytes_bad_input() {
        let _pk = secp256r1::ecdsa_raw_public_key_from_64_bytes(x"11");
    }
}
