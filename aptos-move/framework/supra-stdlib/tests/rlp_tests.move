#[test_only]
module supra_std::rlp_tests {

    use std::features;
    use std::vector;
    use supra_std::rlp;
    use supra_std::rlp::{decode_list_scalar, encode_list_scalar, encode, decode, encode_list_byte_array,
        decode_list_byte_array
    };

    fun prepare_env(supra_framework: &signer) {
        let flag = vector[features::get_supra_rlp_feature()];
        features::change_feature_flags_for_testing(
            supra_framework, flag, vector::empty<u64>()
        );
    }

    #[test]
    #[expected_failure(abort_code = 0x1, location = rlp)]
    public fun test_rlp_encode_feature_disabled() {
        let boolean_val = true;
        let _encoded = encode(boolean_val);
    }

    //
    // 1) Test encode_bool / decode_bool
    //
    #[test(supra_framework = @supra_framework)]
    fun test_bool(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases = vector[true, false];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 1000 + i);
            i = i + 1;
        };
    }

    //
    // 2) Test encode_u8 / decode_u8
    //
    #[test(supra_framework = @supra_framework)]
    fun test_u8(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases: vector<u8> = vector[0, 1, 42, 255];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 2000 + i);
            i = i + 1;
        };
    }

    //
    // 3) Test encode_u16 / decode_u16
    //
    #[test(supra_framework = @supra_framework)]
    fun test_u16(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases: vector<u16> = vector[0, 1, 42, 65535];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 3000 + i);
            i = i + 1;
        };
    }

    //
    // 4) Test encode_u32 / decode_u32
    //
    #[test(supra_framework = @supra_framework)]
    fun test_u32(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases: vector<u32> = vector[
            0,
            1,
            42,
            4294967295 // (2^32 - 1)
        ];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 4000 + i);
            i = i + 1;
        };
    }

    //
    // 5) Test encode_u64 / decode_u64
    //
    #[test(supra_framework = @supra_framework)]
    fun test_u64(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases: vector<u64> = vector[
            0,
            1,
            42,
            9999999999,
            18446744073709551615 // (2^64 - 1)
        ];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 5000 + i);
            i = i + 1;
        };
    }

    //
    // 6) Test encode_u128 / decode_u128
    //
    #[test(supra_framework = @supra_framework)]
    fun test_u128(supra_framework: signer) {
        prepare_env(&supra_framework);
        let cases: vector<u128> = vector[
            0,
            1,
            123456789012345678901234567890,
            340282366920938463463374607431768211455 // (2^128 - 1)
        ];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 6000 + i);
            i = i + 1;
        };
    }

    //
    // 8) Test encode_address / decode_address
    //
    #[test(supra_framework = @supra_framework)]
    fun test_address(supra_framework: signer) {
        prepare_env(&supra_framework);
        // Some representative addresses
        let addr1 = @0x0;
        let addr2 = @0x1;
        let addr3 = @0x1234;
        let addr4 = @0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
        let addr5 = @0x123456789ABCDEF0123456789ABCDEF0;

        let addresses: vector<address> = vector[addr1, addr2, addr3, addr4, addr5];
        let len = vector::length(&addresses);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&addresses, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 8000 + i);
            i = i + 1;
        };
    }

    //
    // 9) Test encode_bytes / decode_bytes
    //
    #[test(supra_framework = @supra_framework)]
    fun test_bytes(supra_framework: signer) {
        prepare_env(&supra_framework);
        let empty = b"";
        let single_byte = b"\xAB";
        let short_bytes = b"Hello RLP!";
        let random_hex = x"DEADBEEF";
        let longer = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let cases: vector<vector<u8>> =  vector[
            empty,
            single_byte,
            short_bytes,
            random_hex,
            longer
        ];
        let len = vector::length(&cases);

        let i = 0;
        while (i < len) {
            let orig = *vector::borrow(&cases, i);
            let encoded = encode(orig);
            let decoded = decode(encoded);
            assert!(decoded == orig, 9000 + i);
            i = i + 1;
        };
    }

    //
    // 10) Test encode_list / decode_list
    //
    #[test(supra_framework = @supra_framework)]
    fun test_list(supra_framework: signer) {
        prepare_env(&supra_framework);
        let u8_list: vector<u8> =  vector[1, 2, 3];
        let encoded = encode_list_scalar<u8>(u8_list);
        let decoded: vector<u8> = decode_list_scalar<u8>(encoded);
        assert!(decoded == u8_list, 10000);

        let u64_list: vector<u64> =  vector[1, 2, 3];
        let encoded = encode_list_scalar<u64>(u64_list);
        let decoded: vector<u64> = decode_list_scalar<u64>(encoded);
        assert!(decoded == u64_list, 10001);

        let bool_list: vector<bool> =  vector[true, false, true];
        let encoded = encode_list_scalar<bool>(bool_list);
        let decoded: vector<bool> = decode_list_scalar<bool>(encoded);
        assert!(decoded == bool_list, 10002);

        let addr1 = @0x0;
        let addr2 = @0x1;
        let addr3 = @0x1234;
        let addr4 = @0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
        let addr5 = @0x123456789ABCDEF0123456789ABCDEF0;
        let adress_list: vector<address> = vector[addr1, addr2, addr3, addr4, addr5];

        let encoded = encode_list_scalar<address>(adress_list);
        let decoded: vector<address> = decode_list_scalar<address>(encoded);
        assert!(decoded == adress_list, 10003);
    }

    #[test(supra_framework = @supra_framework)]
    fun test_list_bytes(supra_framework: signer) {
        prepare_env(&supra_framework);
        let empty = b"";
        let single_byte = b"\xAB";
        let short_bytes = b"Hello RLP!";
        let random_hex = x"DEADBEEF";
        let longer = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let orig: vector<vector<u8>> =  vector[
            empty,
            single_byte,
            short_bytes,
            random_hex,
            longer
        ];

        let encoded = encode_list_byte_array(orig);
        let decoded = decode_list_byte_array(encoded);
        assert!(decoded == orig, 11000);
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x1, location = rlp)]
    fun test_decode_u8_with_invalid_data(supra_framework: signer) {
        prepare_env(&supra_framework);
        let invalid_data = b"\xDE\xAD\xBE\xEF"; // random bytes, not valid RLP
        let _ = decode<vector<u8>>(invalid_data);
        // Should abort.
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x1, location = rlp)]
    fun test_decode_u64_with_empty_data(supra_framework: signer) {
        prepare_env(&supra_framework);
        // Empty data is definitely not valid RLP for a u64
        let invalid_data = b"";
        let _ = decode<u64>(invalid_data);
        // Should abort.
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x1, location = rlp)]
    fun test_decode_address_with_invalid_data(supra_framework: signer) {
        prepare_env(&supra_framework);
        let invalid_data = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz012"; // random bytes, not valid RLP
        let _ = decode<address>(invalid_data);
        // Should abort.
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x3, location = rlp)]
    fun test_encode_with_unsupported_type(supra_framework: signer) {
        prepare_env(&supra_framework);
        let invalid_data = vector[1,2,3];
        let _ = encode<vector<u128>>(invalid_data);
        // Should abort.
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x3, location = rlp)]
    fun test_encode_list_with_unsupported_type(supra_framework: signer) {
        prepare_env(&supra_framework);
        let invalid_data = b"1234";
        let _ = encode_list_scalar<vector<u8>>(vector[invalid_data]);
        // Should abort.
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure( abort_code = 0x3, location = rlp)]
    fun test_decode_list_with_unsupported_type(supra_framework: signer) {
        prepare_env(&supra_framework);
        prepare_env(&supra_framework);
        let invalid_data = b"1234"; // random bytes, not valid RLP
        let _ = decode_list_scalar<vector<u8>>(invalid_data);
        // Should abort.
    }
}
