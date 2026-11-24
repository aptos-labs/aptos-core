// `from_bcs::from_bytes` of a function type, fed the bytes `bcs::to_bytes` produces for
// `0x42::target::add1`. Checks that the decoded function can be called, and that decoding
// succeeds on its own when the value is dropped without a call.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x42::target {
    public fun add1(value: u64): u64 {
        value + 1
    }
}
module 0x42::main {
    fun add1_bytes(): vector<u8> {
        x"0501000000000000000000000000000000000000000000000000000000000000000042067461726765740461646431000000000000000000"
    }

    public fun decode_and_call(): u64 {
        let add_one = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop + store>(add1_bytes());
        add_one(1)
    }

    public fun decode_only(): u64 {
        let _decoded = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop + store>(add1_bytes());
        7
    }
}

// RUN: execute 0x42::main::decode_and_call
// CHECK: results: 2

// RUN: execute 0x42::main::decode_only
// CHECK: results: 7
