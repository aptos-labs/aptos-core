/// base64 support
module aptos_std::aptos_base64 {
    native public fun base64_encode(bytes: vector<u8>): vector<u8>;

    native public fun base64_decode(bytes: vector<u8>): vector<u8>;


    //
    // Testing
    //
    #[test]
    fun base64_test() {
        let input = b"hello";

        let base64_encode_output = base64_encode(input);
        let base64_decode_output = base64_decode(base64_encode_output);

        assert!(base64_encode_output == vector[97, 71, 86, 115, 98, 71, 56, 61], 1);
        assert!(base64_decode_output == vector[104, 101, 108, 108, 111], 1);
    }
}
