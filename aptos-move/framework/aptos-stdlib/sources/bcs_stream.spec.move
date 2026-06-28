spec aptos_std::bcs_stream {
    spec module {
        pragma verify = true;
    }

    /// Constructing a new stream simply pairs the buffer with cursor 0.
    spec new(data: vector<u8>): BCSStream {
        pragma opaque;
        aborts_if false;
        ensures result == BCSStream { data, cur: 0 };
    }

    /// `has_remaining` is a pure read: returns whether the cursor is still within bounds.
    spec has_remaining(stream: &mut BCSStream): bool {
        pragma opaque;
        aborts_if false;
        ensures result == (stream.cur < len(stream.data));
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur);
    }

    /// Reads one byte and advances the cursor by 1.
    spec deserialize_u8(stream: &mut BCSStream): u8 {
        pragma opaque;
        aborts_if stream.cur >= len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (1 as u64);
        ensures result == old(stream.data)[old(stream.cur)];
    }

    /// Reads one byte and decodes it as bool; aborts if byte ∉ {0, 1}.
    spec deserialize_bool(stream: &mut BCSStream): bool {
        pragma opaque;
        aborts_if stream.cur >= len(stream.data);
        let byte = stream.data[stream.cur];
        aborts_if byte != (0 as u8) && byte != (1 as u8);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (1 as u64);
        ensures result == (old(stream.data)[old(stream.cur)] == (1 as u8));
    }

    // Bitwise-using deserializers below: bv8 vs non-bv8 vector encoding conflict; partial specs.
    spec deserialize_address(stream: &mut BCSStream): address {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (32 as u64) > MAX_U64;
        aborts_if stream.cur + (32 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (32 as u64);
    }

    spec deserialize_u16(stream: &mut BCSStream): u16 {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (2 as u64) > MAX_U64;
        aborts_if stream.cur + (2 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (2 as u64);
    }

    spec deserialize_u32(stream: &mut BCSStream): u32 {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (4 as u64) > MAX_U64;
        aborts_if stream.cur + (4 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (4 as u64);
    }

    spec deserialize_u64(stream: &mut BCSStream): u64 {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (8 as u64) > MAX_U64;
        aborts_if stream.cur + (8 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (8 as u64);
    }

    spec deserialize_u128(stream: &mut BCSStream): u128 {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (16 as u64) > MAX_U64;
        aborts_if stream.cur + (16 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (16 as u64);
    }

    spec deserialize_u256(stream: &mut BCSStream): u256 {
        pragma verify = false;
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur + (32 as u64) > MAX_U64;
        aborts_if stream.cur + (32 as u64) > len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur == old(stream.cur) + (32 as u64);
    }

    spec deserialize_uleb128(stream: &mut BCSStream): u64 {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur >= len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur > old(stream.cur);
        ensures stream.cur <= len(stream.data);
    }

    spec deserialize_string(stream: &mut BCSStream): String {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if stream.cur >= len(stream.data);
        ensures stream.data == old(stream.data);
        ensures stream.cur > old(stream.cur);
    }

    spec deserialize_u256_entry(data: vector<u8>, cursor: u64) {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if cursor + (32 as u64) > MAX_U64;
    }
}
