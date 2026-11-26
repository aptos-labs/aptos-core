// `from_bcs::from_bytes` of a `vector<Entry>`, where `Entry` is a u8, a byte vector and a u64
// vector. Every field of every element is read back, including an element whose vectors are
// empty. A cut final element aborts with code 65537.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x42::main {
    struct Entry has drop {
        tag: u8,
        data: vector<u8>,
        counts: vector<u64>,
    }

    // [Entry { 7, "hi", [1, 2] }, Entry { 9, "", [] }, Entry { 11, "xyz", [3] }]:
    // 03 | 07 026869 02 0100000000000000 0200000000000000 | 09 00 00 |
    // 0b 0378797a 01 0300000000000000
    fun encoding(): vector<u8> {
        x"030702686902010000000000000002000000000000000900000b0378797a010300000000000000"
    }

    public fun decode(): (u64, u8, vector<u8>, vector<u64>, u8, vector<u8>, vector<u64>, u8, vector<u8>, vector<u64>) {
        let entries = 0x1::from_bcs::from_bytes<vector<Entry>>(encoding());
        let Entry { tag: tag2, data: data2, counts: counts2 } = entries.pop_back();
        let Entry { tag: tag1, data: data1, counts: counts1 } = entries.pop_back();
        let Entry { tag: tag0, data: data0, counts: counts0 } = entries.pop_back();
        (entries.length() + 3, tag0, data0, counts0, tag1, data1, counts1, tag2, data2, counts2)
    }

    // The last byte of the third element's u64 is missing.
    public fun decode_cut(): u64 {
        let bytes = encoding();
        bytes.pop_back();
        0x1::from_bcs::from_bytes<vector<Entry>>(bytes).length()
    }
}

// RUN: execute 0x42::main::decode
// CHECK: results: 3, 7, 0x6869, [1, 2], 9, 0x, [], 11, 0x78797a, [3]

// RUN: execute 0x42::main::decode_cut
// CHECK: aborted: code 65537 in 0x1::from_bcs
