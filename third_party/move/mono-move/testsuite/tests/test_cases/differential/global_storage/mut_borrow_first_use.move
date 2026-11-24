// Resources holding vectors, committed by a step that runs on V1 only. The next step is the
// first MonoMove runs on these types: it grows them through mutable borrows, with collections
// forced while one stays borrowed, and a last step reads them back.

// RUN: publish
module 0x42::first_use {
    use std::vector;

    struct Inner has store { data: vector<u64> }
    struct Holder has key { data: vector<u64>, list: vector<Inner> }
    enum Slot has key { Empty, Full { values: vector<u64>, inner: Inner } }

    public fun publish(account: signer) {
        move_to(&account, Holder { data: vector[1, 2], list: vector[Inner { data: vector[3] }] });
        move_to(&account, Slot::Full { values: vector[4], inner: Inner { data: vector[5, 6] } });
    }

    public fun grow(addr: address, count: u64) {
        let holder = borrow_global_mut<Holder>(addr);
        let index = 0;
        while (index < count) {
            0x0::test_utils::force_gc();
            vector::push_back(&mut holder.data, index + 10);
            vector::push_back(&mut holder.list, Inner { data: vector[index + 20] });
            match (borrow_global_mut<Slot>(addr)) {
                Slot::Full { values, inner } => {
                    vector::push_back(values, index + 30);
                    vector::push_back(&mut inner.data, index + 40);
                },
                Slot::Empty => {},
            };
            index = index + 1;
        };
    }

    public fun read(addr: address): vector<u64> {
        let holder = borrow_global<Holder>(addr);
        let out = holder.data;
        vector::for_each_ref(&holder.list, |inner| vector::append(&mut out, inner.data));
        match (borrow_global<Slot>(addr)) {
            Slot::Full { values, inner } => {
                vector::append(&mut out, *values);
                vector::append(&mut out, inner.data);
            },
            Slot::Empty => {},
        };
        out
    }
}

// RUN: execute 0x42::first_use::publish --args 0x42 --commit
// CHECK-V1: writes: create 0x42::first_use::Holder at 0x42 0x020100000000000000020000000000000001010300000000000000; create 0x42::first_use::Slot at 0x42 0x010104000000000000000205000000000000000600000000000000

// RUN: execute 0x42::first_use::grow --args 0x42, 3 --commit
// CHECK: writes: modify 0x42::first_use::Holder at 0x42 0x05010000000000000002000000000000000a000000000000000b000000000000000c0000000000000004010300000000000000011400000000000000011500000000000000011600000000000000; modify 0x42::first_use::Slot at 0x42 0x010404000000000000001e000000000000001f0000000000000020000000000000000505000000000000000600000000000000280000000000000029000000000000002a00000000000000
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::first_use::read --args 0x42
// CHECK: results: [1, 2, 10, 11, 12, 3, 20, 21, 22, 4, 30, 31, 32, 5, 6, 40, 41, 42]
