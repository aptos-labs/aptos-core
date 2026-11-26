// Enums whose variant fields use types from another module. The --print step cannot
// resolve those types, so it records the enums without layouts and execution has to build
// them; without --print this path is not reached. Both VMs must agree on the BCS bytes of
// every variant, equality, commit, reload, modify, and the bytes and per-element equality
// of a vector of `E2` built across collections.

// RUN: publish --print(micro-ops)
module 0x42::leaf {
    struct S has copy, drop, store { x: u64 }
    public fun mk(value: u64): S { S { x: value } }
}
module 0x42::mid {
    use std::string::{Self, String};
    use 0x42::leaf::{Self, S};

    // Other-module types appear only as vector elements.
    enum E2 has copy, drop, store { A { v: vector<String> }, B { w: vector<S>, n: u8 } }
    // Other-module types held directly as fields.
    enum E1 has copy, drop, store { A { s: String }, B { w: S }, C }
    struct Holder has key, drop { e1: E1, e2: E2 }

    public fun mk2(sel: u8): E2 {
        if (sel == 0) E2::A { v: vector[string::utf8(b"hi"), string::utf8(b"")] }
        else E2::B { w: vector[leaf::mk(7), leaf::mk(8)], n: sel }
    }
    public fun mk1(sel: u8): E1 {
        if (sel == 0) E1::A { s: string::utf8(b"yo") } else if (sel == 1) E1::B { w: leaf::mk(9) } else E1::C
    }

    public fun bytes(): (vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        (
            std::bcs::to_bytes(&mk2(0)),
            std::bcs::to_bytes(&mk2(3)),
            std::bcs::to_bytes(&mk1(0)),
            std::bcs::to_bytes(&mk1(1)),
            std::bcs::to_bytes(&mk1(2)),
        )
    }
    public fun eq(): (bool, bool, bool, bool) {
        (mk2(0) == mk2(0), mk2(3) == mk2(4), mk1(1) == mk1(1), mk1(0) == mk1(2))
    }
    public fun put(account: signer) { move_to(&account, Holder { e1: mk1(1), e2: mk2(3) }) }
    public fun reload(addr: address): (bool, bool) {
        let holder = borrow_global<Holder>(addr);
        (holder.e1 == mk1(1), holder.e2 == mk2(3))
    }
    // Full bytes of the vector, and whether every element equals a fresh rebuild.
    public fun gc_live(count: u64): (vector<u8>, bool) {
        let es = vector[];
        let push_index = 0;
        while (push_index < count) {
            std::vector::push_back(&mut es, mk2(((push_index % 3) as u8)));
            0x0::test_utils::force_gc();
            push_index = push_index + 1;
        };
        let all_equal = true;
        let check_index = 0;
        while (check_index < count) {
            all_equal = all_equal && *std::vector::borrow(&es, check_index) == mk2(((check_index % 3) as u8));
            check_index = check_index + 1;
        };
        (std::bcs::to_bytes(&es), all_equal)
    }

    public fun swap(addr: address) {
        let holder = borrow_global_mut<Holder>(addr);
        holder.e1 = mk1(0);
        holder.e2 = mk2(0);
    }
}

// RUN: execute 0x42::mid::bytes
// CHECK: results: 0x000202686900, 0x01020700000000000000080000000000000003, 0x0002796f, 0x010900000000000000, 0x02

// RUN: execute 0x42::mid::eq
// CHECK: results: true, false, true, false

// RUN: execute 0x42::mid::put --args 0x42 --commit
// CHECK: writes: create 0x42::mid::Holder at 0x42 0x01090000000000000001020700000000000000080000000000000003

// RUN: execute 0x42::mid::reload --args 0x42
// CHECK: results: true, true

// RUN: execute 0x42::mid::gc_live --args 6 --heap-size 1024
// CHECK: results: 0x0600020268690001020700000000000000080000000000000001010207000000000000000800000000000000020002026869000102070000000000000008000000000000000101020700000000000000080000000000000002, true
// CHECK-GC-COUNT: 8

// RUN: execute 0x42::mid::swap --args 0x42 --commit
// CHECK: writes: modify 0x42::mid::Holder at 0x42 0x0002796f000202686900
