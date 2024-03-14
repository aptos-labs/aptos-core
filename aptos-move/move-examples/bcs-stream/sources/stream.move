module bcs_stream::bcs_stream {
    use std::vector;
    use aptos_std::from_bcs;

    const EMALFORMED_DATA: u64 = 1;
    const EOUT_OF_BYTES: u64 = 2;

    struct BCSStream has drop {
        data: vector<u8>,
        cur: u64,
    }

    public fun new(data: vector<u8>): BCSStream {
        BCSStream {
            data,
            cur: 0,
        }
    }

    public fun next_length(stream: &mut BCSStream): u64 {
        let res = 0;
        let shift = 0;

        while (stream.cur < vector::length(&stream.data)) {
            let byte = *vector::borrow(&stream.data, stream.cur);
            stream.cur = stream.cur + 1;

            let val = ((byte & 0x7f) as u64);
            if (((val << shift) >> shift) != val) {
                abort EMALFORMED_DATA
            };
            res = res | (val << shift);

            if ((byte & 0x80) == 0) {
                if (shift > 0 && val == 0) {
                    abort EMALFORMED_DATA
                };
                return res
            };

            shift = shift + 7;
            if (shift > 64) {
                abort EMALFORMED_DATA
            };
        };

        abort EOUT_OF_BYTES
    }

    public fun next_bool(stream: &mut BCSStream): bool {
        assert!(stream.cur < vector::length(&stream.data), EOUT_OF_BYTES);
        let byte = *vector::borrow(&stream.data, stream.cur);
        stream.cur = stream.cur + 1;
        if (byte == 0) {
            false
        }
        else if (byte == 1) {
            true
        }
        else {
            abort EMALFORMED_DATA
        }
    }

    public fun next_address(stream: &mut BCSStream): address {
        let data = &stream.data;
        let cur = stream.cur;

        assert!(cur + 32 <= vector::length(data), EOUT_OF_BYTES);

        let res = from_bcs::to_address(vector::slice(data, cur, cur + 32));
        stream.cur = cur + 32;
        res
    }

    public fun next_u8(stream: &mut BCSStream): u8 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur < vector::length(data), EOUT_OF_BYTES);
        let res = *vector::borrow(data, cur);
        stream.cur = cur + 1;
        res
    }

    public fun next_u16(stream: &mut BCSStream): u16 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 2 <= vector::length(data), EOUT_OF_BYTES);
        let res =
            (*vector::borrow(data, cur) as u16) |
            ((*vector::borrow(data, cur + 1) as u16) << 8)
        ;

        stream.cur = stream.cur + 2;
        res
    }

    public fun next_u32(stream: &mut BCSStream): u32 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 4 <= vector::length(data), EOUT_OF_BYTES);
        let res =
            (*vector::borrow(data, cur) as u32) |
            ((*vector::borrow(data, cur + 1) as u32) << 8) |
            ((*vector::borrow(data, cur + 2) as u32) << 16) |
            ((*vector::borrow(data, cur + 3) as u32) << 24)
        ;

        stream.cur = stream.cur + 4;
        res
    }

    public fun next_u64(stream: &mut BCSStream): u64 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 8 <= vector::length(data), EOUT_OF_BYTES);
        let res = from_bcs::to_u64(vector::slice(data, cur, cur + 8));
        stream.cur = cur + 8;

        res
    }

    public fun next_u128(stream: &mut BCSStream): u128 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 16 <= vector::length(data), EOUT_OF_BYTES);
        let res = from_bcs::to_u128(vector::slice(data, cur, cur + 16));
        stream.cur = cur + 16;

        res
    }

    public fun next_u256(stream: &mut BCSStream): u256 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 32 <= vector::length(data), EOUT_OF_BYTES);
        let res = from_bcs::to_u256(vector::slice(data, cur, cur + 32));
        stream.cur = cur + 32;

        res
    }

    public inline fun next_vector<E>(stream: &mut BCSStream, next_elem: |&mut BCSStream| E): vector<E> {
        let len = next_length(stream);
        let v = vector::empty();

        let i = 0;
        while (i < len) {
            vector::push_back(&mut v, next_elem(stream));
            i = i + 1;
        };

        v
    }

    /*
    public fun next_u64(stream: &mut BCSStream): u64 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 8 <= vector::length(data), EOUT_OF_BYTES);
        let res =
            (*vector::borrow(data, cur) as u64) |
            ((*vector::borrow(data, cur + 1) as u64) << 8) |
            ((*vector::borrow(data, cur + 2) as u64) << 16) |
            ((*vector::borrow(data, cur + 3) as u64) << 24) |
            ((*vector::borrow(data, cur + 4) as u64) << 32) |
            ((*vector::borrow(data, cur + 5) as u64) << 40) |
            ((*vector::borrow(data, cur + 6) as u64) << 48) |
            ((*vector::borrow(data, cur + 7) as u64) << 56)
        ;

        stream.cur = stream.cur + 8;
        res
    }

    public fun next_u128(stream: &mut BCSStream): u128 {
        let data = &mut stream.data;
        let cur = stream.cur;

        assert!(cur + 16 <= vector::length(data), EOUT_OF_BYTES);
        let res =
            (*vector::borrow(data, cur) as u128) |
            ((*vector::borrow(data, cur + 1) as u128) << 8) |
            ((*vector::borrow(data, cur + 2) as u128) << 16) |
            ((*vector::borrow(data, cur + 3) as u128) << 24) |
            ((*vector::borrow(data, cur + 4) as u128) << 32) |
            ((*vector::borrow(data, cur + 5) as u128) << 40) |
            ((*vector::borrow(data, cur + 6) as u128) << 48) |
            ((*vector::borrow(data, cur + 7) as u128) << 56) |
            ((*vector::borrow(data, cur + 8) as u128) << 64) |
            ((*vector::borrow(data, cur + 9) as u128) << 72) |
            ((*vector::borrow(data, cur + 10) as u128) << 80) |
            ((*vector::borrow(data, cur + 11) as u128) << 88) |
            ((*vector::borrow(data, cur + 12) as u128) << 96) |
            ((*vector::borrow(data, cur + 13) as u128) << 104) |
            ((*vector::borrow(data, cur + 14) as u128) << 112) |
            ((*vector::borrow(data, cur + 15) as u128) << 120)
        ;

        stream.cur = stream.cur + 16;
        res
    }
    */
}
