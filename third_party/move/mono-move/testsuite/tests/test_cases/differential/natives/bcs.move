// Differential test for `bcs::to_bytes`.
//
// `bcs::serialized_size` is implemented and registered too, but the bundled
// `std::bcs` predates it (only `to_bytes` is declared) and the module can't be
// redeclared to add it, so it isn't exercised here. It shares `to_bytes`'s
// serialization path, returning the byte length.

// RUN: publish
module 0x1::main {
    use std::bcs;

    struct Pair has drop {
        a: u64,
        b: bool,
    }

    public fun bytes_u64(): vector<u8> {
        bcs::to_bytes(&42u64)
    }

    public fun bytes_bool(): vector<u8> {
        bcs::to_bytes(&true)
    }

    public fun bytes_u16(): vector<u8> {
        bcs::to_bytes(&258u16)
    }

    public fun bytes_bytes(): vector<u8> {
        bcs::to_bytes(&b"abc")
    }

    public fun bytes_address(): vector<u8> {
        bcs::to_bytes(&@0x1)
    }

    public fun bytes_struct(): vector<u8> {
        bcs::to_bytes(&Pair { a: 1, b: true })
    }
}

// RUN: execute 0x1::main::bytes_u64
// CHECK: results: 0x2a00000000000000

// RUN: execute 0x1::main::bytes_bool
// CHECK: results: 0x01

// RUN: execute 0x1::main::bytes_u16
// CHECK: results: 0x0201

// RUN: execute 0x1::main::bytes_bytes
// CHECK: results: 0x03616263

// RUN: execute 0x1::main::bytes_address
// CHECK: results: 0x0000000000000000000000000000000000000000000000000000000000000001

// RUN: execute 0x1::main::bytes_struct
// CHECK: results: 0x010000000000000001
