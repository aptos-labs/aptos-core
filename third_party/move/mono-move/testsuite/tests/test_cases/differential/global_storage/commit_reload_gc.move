// Resources with vector fields, committed by one step and read from storage by later ones:
// checking existence only, growing them through borrow_global_mut with forced collections
// in between, summing them, and committing the enum resource as another variant. The
// publishing steps run on V1 only, so MonoMove first sees these resources in storage.

// RUN: publish
module 0x42::reload {
    use std::vector;

    struct WithVec has key { tag: u64, data: vector<u64> }
    struct Inner has store { data: vector<u64> }
    struct Outer has key { tag: u8, inner: Inner, list: vector<Inner> }
    enum R has key { A { x: u64 }, B { v: vector<u64>, w: Inner } }

    public fun publish(account: signer) {
        move_to(&account, WithVec { tag: 7, data: vector[1, 2] });
        move_to(&account, Outer { tag: 3, inner: Inner { data: vector[4] }, list: vector[Inner { data: vector[10] }] });
        move_to(&account, R::B { v: vector[5, 6], w: Inner { data: vector[7] } });
    }

    public fun only_exists(addr: address): bool {
        exists<WithVec>(addr) && exists<Outer>(addr) && exists<R>(addr)
    }

    public fun grow(addr: address, count: u64) {
        let index = 0;
        while (index < count) {
            vector::push_back(&mut borrow_global_mut<WithVec>(addr).data, index);
            0x0::test_utils::force_gc();
            let outer = borrow_global_mut<Outer>(addr);
            vector::push_back(&mut outer.inner.data, index);
            vector::push_back(&mut outer.list, Inner { data: vector[index, index] });
            0x0::test_utils::force_gc();
            match (borrow_global_mut<R>(addr)) {
                R::B { v: values, w: inner } => {
                    vector::push_back(values, index);
                    vector::push_back(&mut inner.data, index);
                },
                R::A { x: value } => *value = *value + 1,
            };
            index = index + 1;
        };
    }

    fun sum_vec(values: &vector<u64>): u64 {
        let total = 0;
        vector::for_each_ref(values, |elem| total = total + *elem);
        total
    }

    public fun sum(addr: address): u64 {
        let with_vec = borrow_global<WithVec>(addr);
        let total = with_vec.tag + sum_vec(&with_vec.data);
        0x0::test_utils::force_gc();
        let outer = borrow_global<Outer>(addr);
        total = total + (outer.tag as u64) + sum_vec(&outer.inner.data);
        vector::for_each_ref(&outer.list, |inner| total = total + sum_vec(&inner.data));
        0x0::test_utils::force_gc();
        total + match (borrow_global<R>(addr)) {
            R::B { v: values, w: inner } => sum_vec(values) + sum_vec(&inner.data),
            R::A { x: value } => *value,
        }
    }

    public fun lengths(addr: address): vector<u64> {
        let outer = borrow_global<Outer>(addr);
        let enum_lens = match (borrow_global<R>(addr)) {
            R::B { v: values, w: inner } => vector[vector::length(values), vector::length(&inner.data)],
            R::A { x: _ } => vector[],
        };
        let out = vector[vector::length(&borrow_global<WithVec>(addr).data), vector::length(&outer.inner.data), vector::length(&outer.list)];
        vector::append(&mut out, enum_lens);
        out
    }

    public fun to_a(account: signer) {
        let addr = std::signer::address_of(&account);
        let old = move_from<R>(addr);
        let new_x = match (old) {
            R::B { v: values, w: Inner { data } } => vector::length(&values) + vector::length(&data),
            R::A { x: value } => value,
        };
        0x0::test_utils::force_gc();
        move_to(&account, R::A { x: new_x });
    }
}

// RUN: execute 0x42::reload::publish --args 0x42 --commit
// CHECK-V1: writes: create 0x42::reload::Outer at 0x42 0x0301040000000000000001010a00000000000000; create 0x42::reload::R at 0x42 0x010205000000000000000600000000000000010700000000000000; create 0x42::reload::WithVec at 0x42 0x07000000000000000201000000000000000200000000000000

// RUN: execute 0x42::reload::only_exists --args 0x42
// CHECK: results: true

// RUN: execute 0x42::reload::publish --args 0x43 --commit
// CHECK-V1: writes: create 0x42::reload::Outer at 0x43 0x0301040000000000000001010a00000000000000; create 0x42::reload::R at 0x43 0x010205000000000000000600000000000000010700000000000000; create 0x42::reload::WithVec at 0x43 0x07000000000000000201000000000000000200000000000000

// The first access to the resources at 0x43 is a mutable borrow.
// RUN: execute 0x42::reload::grow --args 0x43, 3 --heap-size 4096 --commit
// CHECK: writes: modify 0x42::reload::Outer at 0x43 0x0304040000000000000000000000000000000100000000000000020000000000000004010a00000000000000020000000000000000000000000000000002010000000000000001000000000000000202000000000000000200000000000000; modify 0x42::reload::R at 0x43 0x010505000000000000000600000000000000000000000000000001000000000000000200000000000000040700000000000000000000000000000001000000000000000200000000000000; modify 0x42::reload::WithVec at 0x43 0x07000000000000000501000000000000000200000000000000000000000000000001000000000000000200000000000000
// CHECK-GC-COUNT: 6

// RUN: execute 0x42::reload::grow --args 0x42, 20 --heap-size 4096 --commit
// CHECK: writes: modify 0x42::reload::Outer at 0x42 0x0315040000000000000000000000000000000100000000000000020000000000000003000000000000000400000000000000050000000000000006000000000000000700000000000000080000000000000009000000000000000a000000000000000b000000000000000c000000000000000d000000000000000e000000000000000f00000000000000100000000000000011000000000000001200000000000000130000000000000015010a000000000000000200000000000000000000000000000000020100000000000000010000000000000002020000000000000002000000000000000203000000000000000300000000000000020400000000000000040000000000000002050000000000000005000000000000000206000000000000000600000000000000020700000000000000070000000000000002080000000000000008000000000000000209000000000000000900000000000000020a000000000000000a00000000000000020b000000000000000b00000000000000020c000000000000000c00000000000000020d000000000000000d00000000000000020e000000000000000e00000000000000020f000000000000000f000000000000000210000000000000001000000000000000021100000000000000110000000000000002120000000000000012000000000000000213000000000000001300000000000000; modify 0x42::reload::R at 0x42 0x01160500000000000000060000000000000000000000000000000100000000000000020000000000000003000000000000000400000000000000050000000000000006000000000000000700000000000000080000000000000009000000000000000a000000000000000b000000000000000c000000000000000d000000000000000e000000000000000f00000000000000100000000000000011000000000000001200000000000000130000000000000015070000000000000000000000000000000100000000000000020000000000000003000000000000000400000000000000050000000000000006000000000000000700000000000000080000000000000009000000000000000a000000000000000b000000000000000c000000000000000d000000000000000e000000000000000f000000000000001000000000000000110000000000000012000000000000001300000000000000; modify 0x42::reload::WithVec at 0x42 0x0700000000000000160100000000000000020000000000000000000000000000000100000000000000020000000000000003000000000000000400000000000000050000000000000006000000000000000700000000000000080000000000000009000000000000000a000000000000000b000000000000000c000000000000000d000000000000000e000000000000000f000000000000001000000000000000110000000000000012000000000000001300000000000000
// CHECK-GC-COUNT: 40

// RUN: execute 0x42::reload::sum --args 0x42 --heap-size 2048
// CHECK: results: 1185
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::reload::lengths --args 0x42
// CHECK: results: [22, 21, 21, 22, 21]

// RUN: execute 0x42::reload::to_a --args 0x42 --heap-size 2048 --commit
// CHECK: writes: modify 0x42::reload::R at 0x42 0x002b00000000000000
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::reload::sum --args 0x42 --heap-size 2048
// CHECK: results: 830
// CHECK-GC-COUNT: 2
