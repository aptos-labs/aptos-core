// Write-set bytes for vector element shapes and nested enums, then a reload
// and an in-place update of each, with collections forced while the reloaded
// resource is borrowed.

// RUN: publish
module 0x42::vshapes {
    use std::option::{Self, Option};
    use std::vector;

    struct Inner has copy, drop, store { x: u8, y: u64 }
    enum Leaf has copy, drop, store { A, B { v: vector<u16> } }
    enum Node has copy, drop, store { One { l: Leaf }, Two { l: Leaf, r: Leaf, w: Inner } }

    struct Vecs has key, drop {
        addrs: vector<address>,
        wides: vector<u128>,
        flags: vector<bool>,
        negs: vector<i16>,
        leaves: vector<Leaf>,
        grid: vector<vector<Inner>>,
        maybe: Option<Inner>,
        node: Node,
    }

    public fun put(account: signer) {
        move_to(&account, Vecs {
            addrs: vector[@0x1, @0xabc],
            wides: vector[1, 1 << 127],
            flags: vector[true, false, true],
            negs: vector[-1, 256, -32768],
            leaves: vector[Leaf::A, Leaf::B { v: vector[7, 8] }],
            grid: vector[vector[], vector[Inner { x: 1, y: 2 }]],
            maybe: option::some(Inner { x: 3, y: 4 }),
            node: Node::Two { l: Leaf::A, r: Leaf::B { v: vector[9] }, w: Inner { x: 5, y: 6 } },
        })
    }

    public fun read(addr: address): vector<u64> {
        let vecs = borrow_global<Vecs>(addr);
        let out = vector[
            vector::length(&vecs.addrs),
            (*vector::borrow(&vecs.wides, 1) >> 120 as u64),
            if (*vector::borrow(&vecs.flags, 2)) 1 else 0,
            ((*vector::borrow(&vecs.negs, 2) + 32767 + 1) as u64),
        ];
        match (vector::borrow(&vecs.leaves, 1)) {
            Leaf::B { v: values } => vector::push_back(&mut out, (*vector::borrow(values, 1) as u64)),
            Leaf::A => vector::push_back(&mut out, 0),
        };
        vector::push_back(&mut out, vector::borrow(vector::borrow(&vecs.grid, 1), 0).y);
        vector::push_back(&mut out, option::borrow(&vecs.maybe).y);
        match (&vecs.node) {
            Node::Two { l: _, r: Leaf::B { v: values }, w: inner } => {
                vector::push_back(&mut out, (*vector::borrow(values, 0) as u64));
                vector::push_back(&mut out, inner.y);
            },
            _ => vector::push_back(&mut out, 0),
        };
        out
    }

    public fun update(addr: address) {
        let vecs = borrow_global_mut<Vecs>(addr);
        0x0::test_utils::force_gc();
        vector::push_back(&mut vecs.addrs, @0xdef);
        *vector::borrow_mut(&mut vecs.flags, 0) = false;
        vector::push_back(&mut vecs.negs, -2);
        *vector::borrow_mut(&mut vecs.leaves, 0) = Leaf::B { v: vector[] };
        0x0::test_utils::force_gc();
        vector::push_back(vector::borrow_mut(&mut vecs.grid, 0), Inner { x: 9, y: 9 });
        option::extract(&mut vecs.maybe);
        vecs.node = Node::One { l: Leaf::A };
    }
}

// RUN: execute 0x42::vshapes::put --args 0x42 --commit
// CHECK: writes: create 0x42::vshapes::Vecs at 0x42 0x0200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000abc0201000000000000000000000000000000000000000000000000000000000000800301000103ffff00010080020001020700080002000101020000000000000001030400000000000000010001010900050600000000000000

// RUN: execute 0x42::vshapes::read --args 0x42
// CHECK: results: [2, 128, 1, 0, 8, 2, 4, 9, 6]

// RUN: execute 0x42::vshapes::update --args 0x42 --heap-size 1024 --commit
// CHECK: writes: modify 0x42::vshapes::Vecs at 0x42 0x0300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000abc0000000000000000000000000000000000000000000000000000000000000def0201000000000000000000000000000000000000000000000000000000000000800300000104ffff00010080feff020100010207000800020109090000000000000001010200000000000000000000
// CHECK-GC-COUNT: 2
