/// This module enables the deserialization of BCS-formatted byte arrays into Move primitive types.
/// Deserialization Strategies:
/// - Per-Byte Deserialization: Employed for most types to ensure lower gas consumption, this method processes each byte
///   individually to match the length and type requirements of target Move types.
/// - Exception: For the `deserialize_address` function, the function-based approach from `velor_std::from_bcs` is used
///   due to type constraints, even though it is generally more gas-intensive.
/// - This can be optimized further by introducing native vector slices.
/// Application:
/// - This deserializer is particularly valuable for processing BCS serialized data within Move modules,
///   especially useful for systems requiring cross-chain message interpretation or off-chain data verification.
module velor_std::bcs_stream {
    use std::error;
    use std::vector;
    use std::option::{Self, Option};
    use std::string::{Self, String};

    use velor_std::from_bcs;

    /// The data does not fit the expected format.
    const EMALFORMED_DATA: u64 = 1;
    /// There are not enough bytes to deserialize for the given type.
    const EOUT_OF_BYTES: u64 = 2;

    struct BCSStream has drop {
        /// Byte buffer containing the serialized data.
        data: vector<u8>,
        /// Cursor indicating the current position in the byte buffer.
        cur: u64,
    }

    /// Constructs a new BCSStream instance from the provided byte array.
    public fun new(data: vector<u8>): BCSStream {
        BCSStream {
            data,
            cur: 0,
        }
    }

    public fun has_remaining(stream: &mut BCSStream): bool {
        stream.cur < stream.data.length()
    }

    /// Deserializes a ULEB128-encoded integer from the stream.
    /// In the BCS format, lengths of vectors are represented using ULEB128 encoding.
    public fun deserialize_uleb128(stream: &mut BCSStream): u64 {
        let res = 0;
        let shift = 0;

        while (stream.cur < stream.data.length()) {
            let byte = stream.data[stream.cur];
            stream.cur += 1;

            let val = ((byte & 0x7f) as u64);
            if (((val << shift) >> shift) != val) {
                abort error::invalid_argument(EMALFORMED_DATA)
            };
            res |= (val << shift);

            if ((byte & 0x80) == 0) {
                if (shift > 0 && val == 0) {
                    abort error::invalid_argument(EMALFORMED_DATA)
                };
                return res
            };

            shift += 7;
            if (shift > 64) {
                abort error::invalid_argument(EMALFORMED_DATA)
            };
        };

        abort error::out_of_range(EOUT_OF_BYTES)
    }

    /// Deserializes a `bool` value from the stream.
    public fun deserialize_bool(stream: &mut BCSStream): bool {
        assert!(stream.cur < stream.data.length(), error::out_of_range(EOUT_OF_BYTES));
        let byte = stream.data[stream.cur];
        stream.cur += 1;
        if (byte == 0) {
            false
        } else if (byte == 1) {
            true
        } else {
            abort error::invalid_argument(EMALFORMED_DATA)
        }
    }

    /// Deserializes an `address` value from the stream.
    /// 32-byte `address` values are serialized using little-endian byte order.
    /// This function utilizes the `to_address` function from the `velor_std::from_bcs` module,
    /// because the Move type system does not permit per-byte referencing of addresses.
    public fun deserialize_address(stream: &mut BCSStream): address {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 32 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res = from_bcs::to_address(data.slice(cur, cur + 32));

        stream.cur = cur + 32;
        res
    }

    /// Deserializes a `u8` value from the stream.
    /// 1-byte `u8` values are serialized using little-endian byte order.
    public fun deserialize_u8(stream: &mut BCSStream): u8 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur < data.length(), error::out_of_range(EOUT_OF_BYTES));

        let res = data[cur];

        stream.cur = cur + 1;
        res
    }

    /// Deserializes a `u16` value from the stream.
    /// 2-byte `u16` values are serialized using little-endian byte order.
    public fun deserialize_u16(stream: &mut BCSStream): u16 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 2 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res =
            (data[cur] as u16) |
                ((data[cur + 1] as u16) << 8)
        ;

        stream.cur += 2;
        res
    }

    /// Deserializes a `u32` value from the stream.
    /// 4-byte `u32` values are serialized using little-endian byte order.
    public fun deserialize_u32(stream: &mut BCSStream): u32 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 4 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res =
            (data[cur] as u32) |
                ((data[cur + 1] as u32) << 8) |
                ((data[cur + 2] as u32) << 16) |
                ((data[cur + 3] as u32) << 24)
        ;

        stream.cur += 4;
        res
    }

    /// Deserializes a `u64` value from the stream.
    /// 8-byte `u64` values are serialized using little-endian byte order.
    public fun deserialize_u64(stream: &mut BCSStream): u64 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 8 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res =
            (data[cur] as u64) |
                ((data[cur + 1] as u64) << 8) |
                ((data[cur + 2] as u64) << 16) |
                ((data[cur + 3] as u64) << 24) |
                ((data[cur + 4] as u64) << 32) |
                ((data[cur + 5] as u64) << 40) |
                ((data[cur + 6] as u64) << 48) |
                ((data[cur + 7] as u64) << 56)
        ;

        stream.cur += 8;
        res
    }

    /// Deserializes a `u128` value from the stream.
    /// 16-byte `u128` values are serialized using little-endian byte order.
    public fun deserialize_u128(stream: &mut BCSStream): u128 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 16 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res =
            (data[cur] as u128) |
                ((data[cur + 1] as u128) << 8) |
                ((data[cur + 2] as u128) << 16) |
                ((data[cur + 3] as u128) << 24) |
                ((data[cur + 4] as u128) << 32) |
                ((data[cur + 5] as u128) << 40) |
                ((data[cur + 6] as u128) << 48) |
                ((data[cur + 7] as u128) << 56) |
                ((data[cur + 8] as u128) << 64) |
                ((data[cur + 9] as u128) << 72) |
                ((data[cur + 10] as u128) << 80) |
                ((data[cur + 11] as u128) << 88) |
                ((data[cur + 12] as u128) << 96) |
                ((data[cur + 13] as u128) << 104) |
                ((data[cur + 14] as u128) << 112) |
                ((data[cur + 15] as u128) << 120)
        ;

        stream.cur += 16;
        res
    }

    /// Deserializes a `u256` value from the stream.
    /// 32-byte `u256` values are serialized using little-endian byte order.
    public fun deserialize_u256(stream: &mut BCSStream): u256 {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 32 <= data.length(), error::out_of_range(EOUT_OF_BYTES));
        let res =
            (data[cur] as u256) |
                ((data[cur + 1] as u256) << 8) |
                ((data[cur + 2] as u256) << 16) |
                ((data[cur + 3] as u256) << 24) |
                ((data[cur + 4] as u256) << 32) |
                ((data[cur + 5] as u256) << 40) |
                ((data[cur + 6] as u256) << 48) |
                ((data[cur + 7] as u256) << 56) |
                ((data[cur + 8] as u256) << 64) |
                ((data[cur + 9] as u256) << 72) |
                ((data[cur + 10] as u256) << 80) |
                ((data[cur + 11] as u256) << 88) |
                ((data[cur + 12] as u256) << 96) |
                ((data[cur + 13] as u256) << 104) |
                ((data[cur + 14] as u256) << 112) |
                ((data[cur + 15] as u256) << 120) |
                ((data[cur + 16] as u256) << 128) |
                ((data[cur + 17] as u256) << 136) |
                ((data[cur + 18] as u256) << 144) |
                ((data[cur + 19] as u256) << 152) |
                ((data[cur + 20] as u256) << 160) |
                ((data[cur + 21] as u256) << 168) |
                ((data[cur + 22] as u256) << 176) |
                ((data[cur + 23] as u256) << 184) |
                ((data[cur + 24] as u256) << 192) |
                ((data[cur + 25] as u256) << 200) |
                ((data[cur + 26] as u256) << 208) |
                ((data[cur + 27] as u256) << 216) |
                ((data[cur + 28] as u256) << 224) |
                ((data[cur + 29] as u256) << 232) |
                ((data[cur + 30] as u256) << 240) |
                ((data[cur + 31] as u256) << 248);

        stream.cur += 32;
        res
    }

    /// Deserializes a `u256` value from the stream.
    public entry fun deserialize_u256_entry(data: vector<u8>, cursor: u64) {
        let stream = BCSStream {
            data,
            cur: cursor,
        };
        deserialize_u256(&mut stream);
    }

    /// Deserializes an array of BCS deserializable elements from the stream.
    /// First, reads the length of the vector, which is in uleb128 format.
    /// After determining the length, it then reads the contents of the vector.
    /// The `elem_deserializer` lambda expression is used sequentially to deserialize each element of the vector.
    public inline fun deserialize_vector<E>(stream: &mut BCSStream, elem_deserializer: |&mut BCSStream| E): vector<E> {
        let len = deserialize_uleb128(stream);
        let v = vector::empty();

        for (i in 0..len) {
            v.push_back(elem_deserializer(stream));
        };

        v
    }

    /// Deserializes utf-8 `String` from the stream.
    /// First, reads the length of the String, which is in uleb128 format.
    /// After determining the length, it then reads the contents of the String.
    public fun deserialize_string(stream: &mut BCSStream): String {
        let len = deserialize_uleb128(stream);
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + len <= data.length(), error::out_of_range(EOUT_OF_BYTES));

        let res = string::utf8(data.slice(cur, cur + len));
        stream.cur = cur + len;

        res
    }

    /// Deserializes `Option` from the stream.
    /// First, reads a single byte representing the presence (0x01) or absence (0x00) of data.
    /// After determining the presence of data, it then reads the actual data if present.
    /// The `elem_deserializer` lambda expression is used to deserialize the element contained within the `Option`.
    public inline fun deserialize_option<E>(stream: &mut BCSStream, elem_deserializer: |&mut BCSStream| E): Option<E> {
        let is_data = deserialize_bool(stream);
        if (is_data) {
            option::some(elem_deserializer(stream))
        } else {
            option::none()
        }
    }
}
