// RUN: publish
module 0x1::boxes {
    struct Box<T> { value: T }

    struct Pair<A, B> { first: A, second: B }
}

module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}

module 0x1::main {
    use std::bcs;
    use std::option;
    use 0x1::boxes;

    struct Pair has drop {
        a: u64,
        b: bool,
    }

    struct Wrap has drop {
        v: vector<u8>,
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

    public fun bytes_nested_vec(): vector<u8> {
        bcs::to_bytes(&vector[vector[1u8, 2u8], vector[3u8, 4u8, 5u8]])
    }

    public fun bytes_wrap(): vector<u8> {
        bcs::to_bytes(&Wrap { v: b"hi" })
    }

    public fun size_u64(): u64 {
        bcs::serialized_size(&42u64)
    }

    public fun size_bool(): u64 {
        bcs::serialized_size(&true)
    }

    public fun size_u16(): u64 {
        bcs::serialized_size(&258u16)
    }

    public fun size_bytes(): u64 {
        bcs::serialized_size(&b"abc")
    }

    public fun size_address(): u64 {
        bcs::serialized_size(&@0x1)
    }

    public fun size_struct(): u64 {
        bcs::serialized_size(&Pair { a: 1, b: true })
    }

    public fun size_nested_vec(): u64 {
        bcs::serialized_size(&vector[vector[1u8, 2u8], vector[3u8, 4u8, 5u8]])
    }

    public fun size_wrap(): u64 {
        bcs::serialized_size(&Wrap { v: b"hi" })
    }

    public fun const_size_box_u64(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<u64>>())
    }

    public fun const_size_box_bool(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<bool>>())
    }

    public fun const_size_box_address(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<address>>())
    }

    public fun const_size_pair(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Pair<u64, bool>>())
    }

    public fun const_size_nested(): u64 {
        option::destroy_some(bcs::constant_serialized_size<boxes::Box<boxes::Pair<u64, u128>>>())
    }

    public fun const_size_is_some_box_u64(): bool {
        option::is_some(&bcs::constant_serialized_size<boxes::Box<u64>>())
    }

    public fun const_size_is_some_box_vec(): bool {
        option::is_some(&bcs::constant_serialized_size<boxes::Box<vector<u8>>>())
    }

    public fun const_size_is_some_signer(): bool {
        option::is_some(&bcs::constant_serialized_size<signer>())
    }

    public fun const_size_is_some_box_signer(): bool {
        option::is_some(&bcs::constant_serialized_size<boxes::Box<signer>>())
    }
}

module 0x99::closure_bcs {
    use std::bcs;

    struct Point has copy, drop, store { x: u64, y: u8 }

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    #[persistent]
    fun norm(p: Point, k: u64): u64 {
        p.x + (p.y as u64) + k
    }

    #[persistent]
    fun pick<T: drop>(a: T, b: u64): u64 {
        let _ = a;
        b
    }

    #[persistent]
    fun owner(s: signer, b: u64): u64 {
        let _ = s;
        b
    }

    public fun ser_noncapturing(): vector<u8> {
        let f: |u64, u64|u64 has drop = add;
        bcs::to_bytes(&f)
    }

    public fun ser_capturing(): vector<u8> {
        let f: |u64|u64 has drop = |b| add(7, b);
        bcs::to_bytes(&f)
    }

    public fun ser_vector_capture(): vector<u8> {
        let data = vector[1u8, 2u8, 3u8];
        let f: |u64|u64 has drop = |k| tally(data, k);
        bcs::to_bytes(&f)
    }

    public fun ser_struct_capture(): vector<u8> {
        let p = Point { x: 5, y: 7 };
        let f: |u64|u64 has drop = |k| norm(p, k);
        bcs::to_bytes(&f)
    }

    public fun ser_generic(): vector<u8> {
        let a = true;
        let f: |u64|u64 has drop = |b| pick<bool>(a, b);
        bcs::to_bytes(&f)
    }

    public fun ser_generic_struct(): vector<u8> {
        let p = Point { x: 1, y: 2 };
        let f: |u64|u64 has drop = |b| pick<Point>(p, b);
        bcs::to_bytes(&f)
    }

    public fun ser_signer_capture(addr: address): vector<u8> {
        let s = 0x1::create_signer::create_signer(addr);
        let f: |u64|u64 has drop = |b| owner(s, b);
        bcs::to_bytes(&f)
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

// RUN: execute 0x1::main::bytes_nested_vec
// CHECK: results: 0x0202010203030405

// RUN: execute 0x1::main::bytes_wrap
// CHECK: results: 0x026869

// Each serialized size equals the byte length of the corresponding `to_bytes`.

// RUN: execute 0x1::main::size_u64
// CHECK: results: 8

// RUN: execute 0x1::main::size_bool
// CHECK: results: 1

// RUN: execute 0x1::main::size_u16
// CHECK: results: 2

// RUN: execute 0x1::main::size_bytes
// CHECK: results: 4

// RUN: execute 0x1::main::size_address
// CHECK: results: 32

// RUN: execute 0x1::main::size_struct
// CHECK: results: 9

// RUN: execute 0x1::main::size_nested_vec
// CHECK: results: 8

// RUN: execute 0x1::main::size_wrap
// CHECK: results: 3

// RUN: execute 0x1::main::const_size_box_u64
// CHECK: results: 8

// RUN: execute 0x1::main::const_size_box_bool
// CHECK: results: 1

// RUN: execute 0x1::main::const_size_box_address
// CHECK: results: 32

// RUN: execute 0x1::main::const_size_pair
// CHECK: results: 9

// RUN: execute 0x1::main::const_size_nested
// CHECK: results: 24

// RUN: execute 0x1::main::const_size_is_some_box_u64
// CHECK: results: true

// RUN: execute 0x1::main::const_size_is_some_box_vec
// CHECK: results: false

// RUN: execute 0x1::main::const_size_is_some_signer
// CHECK: results: false

// RUN: execute 0x1::main::const_size_is_some_box_signer
// CHECK: results: false

// RUN: execute 0x99::closure_bcs::ser_noncapturing
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000

// RUN: execute 0x99::closure_bcs::ser_capturing
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000

// RUN: execute 0x99::closure_bcs::ser_vector_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203

// RUN: execute 0x99::closure_bcs::ser_struct_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373046e6f726d0001000000000000000600020201050000000000000007

// RUN: execute 0x99::closure_bcs::ser_generic
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373047069636b010001000000000000000001

// RUN: execute 0x99::closure_bcs::ser_generic_struct
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373047069636b010700000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637305506f696e740001000000000000000600020201010000000000000002

// A captured signer serializes as its address, as V1's bcs natives write it.

// RUN: execute 0x99::closure_bcs::ser_signer_capture --args 0xcafe
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373056f776e657200010000000000000007000000000000000000000000000000000000000000000000000000000000cafe
