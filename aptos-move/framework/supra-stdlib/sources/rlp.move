module supra_std::rlp {

    use std::bcs;
    use std::features;
    use aptos_std::any;
    use aptos_std::type_info;

    /// SUPRA_RLP_ENCODE feature APIs are disabled.
    const ERLP_ENCODE_FEATURE_DISABLED: u64 = 1;


    // Encode/Decode for type T
    // Types supported: bool, u8, u16, u32, u64, u128, address, vector<u8>
    // Attempting to encode any other type results in E_UNSUPPORTED_TYPE error
    public fun encode<T>(x: T): vector<u8> {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        native_rlp_encode(x)
    }

    public fun decode<T>(encoded_rlp: vector<u8>): T {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        native_rlp_decode(encoded_rlp)
    }

    // Encode/Decode for list
    // Type of lists supported: bool, u8, u16, u32, u64, u128, address
    // Attempting to encode any other type results in E_UNSUPPORTED_TYPE error
    public fun encode_list_scalar<T: drop>(data: vector<T>): vector<u8> {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        native_rlp_encode_list_scalar<T>(bcs::to_bytes(&data))
    }

    public fun decode_list_scalar<T>(encoded_rlp: vector<u8>): vector<T> {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        native_rlp_decode_list_scalar<T>(encoded_rlp)
    }

    // Encode/Decode for list of byte arrays: (vec[vec[u8], vec[u8], ..])
    public fun encode_list_byte_array(data: vector<vector<u8>>): vector<u8> {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        native_rlp_encode_list_byte_array(bcs::to_bytes(&data))
    }

    public fun decode_list_byte_array(encoded_rlp: vector<u8>): vector<vector<u8>> {
        assert!(features::supra_rlp_enabled(), ERLP_ENCODE_FEATURE_DISABLED);
        let ser_result = native_rlp_decode_list_byte_array(encoded_rlp);
        let any_ser = any::new(type_info::type_name<vector<vector<u8>>>(), ser_result);
        any::unpack<vector<vector<u8>>>(any_ser)
    }

    //
    // Native functions
    //
    native fun native_rlp_encode<T>(x: T): vector<u8>;
    native fun native_rlp_decode<T>(data: vector<u8>): T;

    native fun native_rlp_encode_list_scalar<T>(x: vector<u8>): vector<u8>;
    native fun native_rlp_decode_list_scalar<T>(data: vector<u8>): vector<T>;

    native fun native_rlp_encode_list_byte_array(x: vector<u8>): vector<u8>;
    native fun native_rlp_decode_list_byte_array(data: vector<u8>): vector<u8>;
}
