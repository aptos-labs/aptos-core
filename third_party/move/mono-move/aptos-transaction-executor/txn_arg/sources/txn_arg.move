/// Deserialization of transaction arguments, run by the VM before an entry
/// function. Never published: the VM serves this module itself.
///
/// The specializer replaces each call to `deserialize<T>` with the function
/// below that deserializes the concrete `T`, so the type walk is
/// monomorphization.
module txn_arg::txn_arg {
    use std::error;
    use std::fixed_point32::{Self, FixedPoint32};
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::fixed_point64::{Self, FixedPoint64};
    use aptos_std::from_bcs;
    use aptos_framework::object::{Self, Object};

    /// The bytes do not encode a value of the expected type.
    const EMALFORMED_DATA: u64 = 1;
    /// The bytes end before the value does.
    const EOUT_OF_BYTES: u64 = 2;
    /// Bytes remain after the value.
    const ETRAILING_BYTES: u64 = 3;
    /// The type has no deserializer. Unreachable: lowering refuses the type.
    const ENOT_DESERIALIZABLE: u64 = 4;

    struct BCSStream has drop {
        data: vector<u8>,
        cur: u64,
    }

    /// Deserializes one transaction argument of type `T` from its BCS bytes.
    public fun deserialize_arg<T>(bytes: vector<u8>): T {
        let len = vector::length(&bytes);
        let stream = BCSStream { data: bytes, cur: 0 };
        let value = deserialize<T>(&mut stream);
        assert!(stream.cur == len, error::invalid_argument(ETRAILING_BYTES));
        value
    }

    /// Deserializes a value of type `T`. The specializer replaces each call
    /// with the deserializer for the concrete `T`; the body is never reached.
    fun deserialize<T>(_stream: &mut BCSStream): T {
        abort error::invalid_argument(ENOT_DESERIALIZABLE)
    }

    fun deserialize_vector<E>(stream: &mut BCSStream): vector<E> {
        let len = deserialize_uleb128(stream);
        let v = vector::empty<E>();
        let i = 0;
        while (i < len) {
            vector::push_back(&mut v, deserialize<E>(stream));
            i = i + 1;
        };
        v
    }

    fun deserialize_option<E>(stream: &mut BCSStream): Option<E> {
        let tag = deserialize_uleb128(stream);
        if (tag == 0) {
            option::none()
        } else if (tag == 1) {
            option::some(deserialize<E>(stream))
        } else {
            abort error::invalid_argument(EMALFORMED_DATA)
        }
    }

    fun deserialize_string(stream: &mut BCSStream): String {
        string::utf8(deserialize_bytes(stream))
    }

    fun deserialize_object<T: key>(stream: &mut BCSStream): Object<T> {
        object::address_to_object<T>(deserialize_address(stream))
    }

    fun deserialize_fixed_point32(stream: &mut BCSStream): FixedPoint32 {
        fixed_point32::create_from_raw_value(deserialize_u64(stream))
    }

    fun deserialize_fixed_point64(stream: &mut BCSStream): FixedPoint64 {
        fixed_point64::create_from_raw_value(deserialize_u128(stream))
    }

    fun deserialize_bytes(stream: &mut BCSStream): vector<u8> {
        let len = deserialize_uleb128(stream);
        let cur = stream.cur;
        assert!(cur + len <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        stream.cur = cur + len;
        vector::slice(&stream.data, cur, cur + len)
    }

    /// Reads a ULEB128 length or enum tag. Public for the generated
    /// deserializers of public structs and enums.
    public fun deserialize_uleb128(stream: &mut BCSStream): u64 {
        let res = 0;
        let shift = 0;
        while (stream.cur < vector::length(&stream.data)) {
            let byte = *vector::borrow(&stream.data, stream.cur);
            stream.cur = stream.cur + 1;
            let val = ((byte & 0x7f) as u64);
            if (((val << shift) >> shift) != val) {
                abort error::invalid_argument(EMALFORMED_DATA)
            };
            res = res | (val << shift);
            if ((byte & 0x80) == 0) {
                if (shift > 0 && val == 0) {
                    abort error::invalid_argument(EMALFORMED_DATA)
                };
                return res
            };
            shift = shift + 7;
            if (shift > 64) {
                abort error::invalid_argument(EMALFORMED_DATA)
            };
        };
        abort error::out_of_range(EOUT_OF_BYTES)
    }

    fun deserialize_bool(stream: &mut BCSStream): bool {
        let byte = deserialize_u8(stream);
        if (byte == 0) {
            false
        } else if (byte == 1) {
            true
        } else {
            abort error::invalid_argument(EMALFORMED_DATA)
        }
    }

    fun deserialize_address(stream: &mut BCSStream): address {
        let cur = stream.cur;
        assert!(cur + 32 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        stream.cur = cur + 32;
        from_bcs::to_address(vector::slice(&stream.data, cur, cur + 32))
    }

    fun deserialize_u8(stream: &mut BCSStream): u8 {
        let cur = stream.cur;
        assert!(cur < vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        stream.cur = cur + 1;
        *vector::borrow(&stream.data, cur)
    }

    fun deserialize_u16(stream: &mut BCSStream): u16 {
        let cur = stream.cur;
        assert!(cur + 2 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        let data = &stream.data;
        let res = (*vector::borrow(data, cur) as u16)
            | ((*vector::borrow(data, cur + 1) as u16) << 8);
        stream.cur = cur + 2;
        res
    }

    fun deserialize_u32(stream: &mut BCSStream): u32 {
        let cur = stream.cur;
        assert!(cur + 4 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        let data = &stream.data;
        let res = (*vector::borrow(data, cur) as u32)
            | ((*vector::borrow(data, cur + 1) as u32) << 8)
            | ((*vector::borrow(data, cur + 2) as u32) << 16)
            | ((*vector::borrow(data, cur + 3) as u32) << 24);
        stream.cur = cur + 4;
        res
    }

    fun deserialize_u64(stream: &mut BCSStream): u64 {
        let cur = stream.cur;
        assert!(cur + 8 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        let data = &stream.data;
        let res = 0u64;
        let i = 0;
        while (i < 8) {
            res = res | ((*vector::borrow(data, cur + i) as u64) << ((8 * i) as u8));
            i = i + 1;
        };
        stream.cur = cur + 8;
        res
    }

    fun deserialize_u128(stream: &mut BCSStream): u128 {
        let cur = stream.cur;
        assert!(cur + 16 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        let data = &stream.data;
        let res = 0u128;
        let i = 0;
        while (i < 16) {
            res = res | ((*vector::borrow(data, cur + i) as u128) << ((8 * i) as u8));
            i = i + 1;
        };
        stream.cur = cur + 16;
        res
    }

    fun deserialize_u256(stream: &mut BCSStream): u256 {
        let cur = stream.cur;
        assert!(cur + 32 <= vector::length(&stream.data), error::out_of_range(EOUT_OF_BYTES));
        let data = &stream.data;
        let res = 0u256;
        let i = 0;
        while (i < 32) {
            res = res | ((*vector::borrow(data, cur + i) as u256) << ((8 * i) as u8));
            i = i + 1;
        };
        stream.cur = cur + 32;
        res
    }
}
