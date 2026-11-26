// BCS of byte vectors whose lengths straddle the one/two-byte ULEB128 boundary (127 and
// 128), nested in a struct with a trailing u16. The encoding is pinned byte for byte,
// compared with bytes built by hand, decoded from the hand-built bytes, and written to
// storage and read back.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x42::lenb {
    use std::bcs;
    use std::vector;

    struct B has key, copy, drop { vs: vector<vector<u8>>, t: u16 }

    fun fill(byte: u8, count: u64): vector<u8> {
        let bytes = vector[];
        let index = 0;
        while (index < count) { vector::push_back(&mut bytes, byte); index = index + 1; };
        bytes
    }

    fun value(): B { B { vs: vector[fill(0x11, 127), fill(0x22, 128)], t: 0x1234 } }

    // 02 7f [11 x 127] 80 01 [22 x 128] 34 12, built without the serializer.
    fun expected(): vector<u8> {
        let out = vector[0x02, 0x7f];
        vector::append(&mut out, fill(0x11, 127));
        vector::append(&mut out, vector[0x80, 0x01]);
        vector::append(&mut out, fill(0x22, 128));
        vector::append(&mut out, vector[0x34, 0x12]);
        out
    }

    public fun encode(): vector<u8> { bcs::to_bytes(&value()) }

    public fun encode_matches(): (bool, u64) {
        let bytes = bcs::to_bytes(&value());
        (bytes == expected(), vector::length(&bytes))
    }

    public fun decode(): (u64, u64, u16, u8, u8, bool) {
        let decoded = 0x1::from_bcs::from_bytes<B>(expected());
        let first = vector::borrow(&decoded.vs, 0);
        let second = vector::borrow(&decoded.vs, 1);
        (vector::length(first), vector::length(second), decoded.t, *vector::borrow(first, 126), *vector::borrow(second, 127), decoded == value())
    }

    public fun put(account: signer) { move_to(&account, value()) }

    public fun reload(addr: address): (u64, u64, u16) {
        let stored = borrow_global<B>(addr);
        (vector::length(vector::borrow(&stored.vs, 0)), vector::length(vector::borrow(&stored.vs, 1)), stored.t)
    }
}

// RUN: execute 0x42::lenb::encode
// CHECK: results: 0x027f11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111800122222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222223412

// RUN: execute 0x42::lenb::encode_matches
// CHECK: results: true, 261

// RUN: execute 0x42::lenb::decode
// CHECK: results: 127, 128, 4660, 17, 34, true

// RUN: execute 0x42::lenb::put --args 0x42 --commit
// CHECK: writes: create 0x42::lenb::B at 0x42 0x027f11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111800122222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222223412

// RUN: execute 0x42::lenb::reload --args 0x42
// CHECK: results: 127, 128, 4660
