// BCS serialization and deserialization must preserve enum tags and fields
// while excluding struct padding from the encoded bytes.
//
// The test prelude omits `from_bcs`; this declaration binds to both VMs' native.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}

module 0x1::main {
    use std::bcs;

    enum E has copy, drop { A { x: u64 }, B { y: bool, z: u16 } }

    // The u64 field requires alignment padding after the first u8 field.
    struct P has copy, drop { a: u8, b: u64, c: u8 }

    public fun rt_enum_a(): u64 {
        let e: E = 0x1::from_bcs::from_bytes<E>(x"002a00000000000000");
        match (e) { E::A { x } => x, E::B { .. } => 0 }
    }

    public fun rt_enum_b(): u64 {
        let e: E = 0x1::from_bcs::from_bytes<E>(x"01010700");
        match (e) { E::A { .. } => 0, E::B { z, .. } => z as u64 }
    }

    // P{a:5, b:42, c:7} serializes without padding: 05 | 2a00000000000000 | 07.
    public fun rt_padded_struct(): u64 {
        let p: P = 0x1::from_bcs::from_bytes<P>(x"052a0000000000000007");
        (p.a as u64) + p.b + (p.c as u64)
    }

    public fun ser_enum_b(): vector<u8> {
        bcs::to_bytes(&E::B { y: true, z: 7 })
    }

    public fun ser_padded(): vector<u8> {
        bcs::to_bytes(&P { a: 5, b: 42, c: 7 })
    }
}

// RUN: execute 0x1::main::rt_enum_a
// CHECK: results: 42

// RUN: execute 0x1::main::rt_enum_b
// CHECK: results: 7

// RUN: execute 0x1::main::rt_padded_struct
// CHECK: results: 54

// RUN: execute 0x1::main::ser_enum_b
// CHECK: results: 0x01010700

// RUN: execute 0x1::main::ser_padded
// CHECK: results: 0x052a0000000000000007
