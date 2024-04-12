#[test_only]
module bcs_stream::tests {
    use bcs_stream::bcs_stream;
    use std::vector;

    struct Bar has drop {
        x: u16,
        y: bool,
    }

    struct Foo has drop {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        f: u256,
        g: bool,
        h: vector<Bar>,
        i: address,
    }

    fun deserialize_Bar(stream: &mut bcs_stream::BCSStream): Bar {
        Bar {
            x: bcs_stream::deserialize_u16(stream),
            y: bcs_stream::deserialize_bool(stream),
        }
    }

    fun deserialize_Foo(stream: &mut bcs_stream::BCSStream): Foo {
        Foo {
            a: bcs_stream::deserialize_u8(stream),
            b: bcs_stream::deserialize_u16(stream),
            c: bcs_stream::deserialize_u32(stream),
            d: bcs_stream::deserialize_u64(stream),
            e: bcs_stream::deserialize_u128(stream),
            f: bcs_stream::deserialize_u256(stream),
            g: bcs_stream::deserialize_bool(stream),
            h: bcs_stream::deserialize_vector(stream, |stream| {
                deserialize_Bar(stream)
            }),
            i: bcs_stream::deserialize_address(stream),
        }
    }

    #[test]
    fun test_struct_all_types() {
        let data = vector::empty();
        vector::append(&mut data, x"01"); // u8
        vector::append(&mut data, x"0200"); // u16
        vector::append(&mut data, x"03000000"); // u32
        vector::append(&mut data, x"0400000000000000"); // u64
        vector::append(&mut data, x"05000000000000000000000000000000"); // u128
        vector::append(&mut data, x"0600000000000000000000000000000000000000000000000000000000000000"); // u256
        vector::append(&mut data, x"01"); // bool
        vector::append(&mut data, x"02010000020001"); // vector
        vector::append(&mut data, x"000000000000000000000000000000000000000000000000000000000000ABCD"); // address

        let stream = bcs_stream::new(data);
        let foo = deserialize_Foo(&mut stream);

        let expected = Foo {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: 5,
            f: 6,
            g: true,
            h: vector[Bar { x: 01, y: false }, Bar { x: 02, y: true }],
            i: @0xABCD,
        };

        assert!(foo == expected, 0);
    }

    #[test]
    fun test_bool_true() {
        let data = x"01";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_bool(&mut stream) == true, 0);
    }

    #[test]
    fun test_bool_false() {
        let data = x"00";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_bool(&mut stream) == false, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x010001, location = bcs_stream::bcs_stream)]
    fun test_bool_invalid() {
        let data = x"02";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_bool(&mut stream);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_bool_out_of_bytes() {
        let data = vector::empty();
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_bool(&mut stream);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_length_no_bytes() {
        let data = vector::empty();
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_uleb128(&mut stream);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_length_no_ending_group() {
        let data = x"808080808080808080";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_uleb128(&mut stream);
    }

    #[test]
    fun test_length_0() {
        let data = x"00";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 0, 0);
    }

    #[test]
    fun test_length_1() {
        let data = x"01";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 1, 0);
    }

    #[test]
    fun test_length_127() {
        let data = x"7f";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 127, 0);
    }

    #[test]
    fun test_length_128() {
        let data = x"8001";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 128, 0);
    }

    #[test]
    fun test_length_130() {
        let data = x"8201";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 130, 0);
    }

    #[test]
    fun test_length_16383() {
        let data = x"ff7f";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_uleb128(&mut stream) == 16383, 0);
    }

    #[test]
    fun test_length_large() {
        let data = x"E1D8F9FC06";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 1872653409, 0);

        let data = x"BEC020";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 532542, 0);

        let data = x"B39AEDD084E15D";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 412352463457587, 0);
    }

    #[test]
    fun test_length_2_pow_63_minus_1() {
        let data = x"FFFFFFFFFFFFFFFF7F";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 9223372036854775807, 0);
    }

    #[test]
    fun test_length_2_pow_63() {
        let data = x"80808080808080808001";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 9223372036854775808, 0);
    }

    #[test]
    fun test_length_2_pow_64_minus_1() {
        let data = x"FFFFFFFFFFFFFFFFFF01";
        let stream = bcs_stream::new(data);
        let len = bcs_stream::deserialize_uleb128(&mut stream);
        assert!(len == 18446744073709551615, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x010001, location = bcs_stream::bcs_stream)]
    fun test_length_2_pow_64() {
        let data = x"80808080808080808002";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_uleb128(&mut stream);
    }

    #[test]
    fun test_vector_simple() {
        let data = x"03010203";
        let stream = bcs_stream::new(data);
        let v = bcs_stream::deserialize_vector(&mut stream, |stream| {
            bcs_stream::deserialize_u8(stream)
        });
        assert!(v == vector[1, 2, 3], 0);
    }

    #[test]
    fun test_vector_empty() {
        let data = x"00";
        let stream = bcs_stream::new(data);
        let v = bcs_stream::deserialize_vector(&mut stream, |stream| {
            bcs_stream::deserialize_u8(stream)
        });
        assert!(v == vector[], 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_vector_not_enough_items() {
        let data = x"FFFFFFFFFFFFFFFFFF01";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_vector(&mut stream, |stream| {
            bcs_stream::deserialize_u8(stream)
        });
    }

    #[test]
    fun test_u8() {
        let data = vector::empty();

        let i = 0;
        while (i < 256) {
            vector::push_back(&mut data, (i as u8));
            i = i + 1;
        };

        let stream = bcs_stream::new(data);
        let i = 0;
        while (i < 256) {
            assert!(bcs_stream::deserialize_u8(&mut stream) == (i as u8), 0);
            i = i + 1;
        }
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u8_out_of_bytes() {
        let data = vector::empty();
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u8(&mut stream);
    }

    #[test]
    fun test_u16() {
        let data = vector::empty();

        let i = 0;
        while (i < 65536) {
            vector::push_back(&mut data, ((i % 256) as u8));
            vector::push_back(&mut data, ((i / 256) as u8));
            i = i + 1;
        };

        let stream = bcs_stream::new(data);
        let i = 0;
        while (i < 65536) {
            assert!(bcs_stream::deserialize_u16(&mut stream) == (i as u16), 0);
            i = i + 1;
        }
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u16_out_of_bytes() {
        let data = x"00";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u16(&mut stream);
    }

    #[test]
    fun test_u32() {
        let data = x"01020304";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_u32(&mut stream) == 0x04030201, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u32_out_of_bytes() {
        let data = x"000000";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u32(&mut stream);
    }

    #[test]
    fun test_u64() {
        let data = x"0102030405060708";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_u64(&mut stream) == 0x0807060504030201, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u64_out_of_bytes() {
        let data = x"00000000000000";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u64(&mut stream);
    }

    #[test]
    fun test_u128() {
        let data = x"01020304050607081112131415161718";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_u128(&mut stream) == 0x18171615141312110807060504030201, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u128_out_of_bytes() {
        let data = x"000000000000000000000000000000";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u128(&mut stream);
    }

    #[test]
    fun test_u256() {
        let data = x"0102030405060708111213141516171821222324252627283132333435363738";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_u256(&mut stream) == 0x3837363534333231282726252423222118171615141312110807060504030201, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_u256_out_of_bytes() {
        let data = x"00000000000000000000000000000000000000000000000000000000000000";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_u256(&mut stream);
    }

    #[test]
    fun test_address() {
        let data = x"0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
        let stream = bcs_stream::new(data);
        assert!(bcs_stream::deserialize_address(&mut stream) == @0x0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF, 0);
    }

    #[test]
    #[expected_failure(abort_code = 0x020002, location = bcs_stream::bcs_stream)]
    fun test_address_too_short() {
        let data = x"0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCD";
        let stream = bcs_stream::new(data);
        bcs_stream::deserialize_address(&mut stream);
    }
}
