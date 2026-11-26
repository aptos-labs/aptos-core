// A forced collection while live values hold empty vector fields that were never pushed
// to: a resource just published, two enum values, a vector of structs, and an emitted
// module event. Each value is read back after the collection.

// RUN: publish
module 0x1::event {
    use std::vector;

    struct MyEvent has drop, store { a: u64, data: vector<u8> }

    native fun write_module_event_to_store<T: drop + store>(msg: T);
    native fun emitted_events<T: drop + store>(): vector<T>;

    fun emit(value: u64, data: vector<u8>) {
        write_module_event_to_store(MyEvent { a: value, data });
    }

    public fun emit_empty_then_gc(value: u64): u64 {
        emit(value, vector[]);
        emit(value + 1, vector[7u8]);
        0x0::test_utils::force_gc();
        let evs = emitted_events<MyEvent>();
        let first = vector::borrow(&evs, 0);
        let second = vector::borrow(&evs, 1);
        vector::length(&evs) * 1000000 + first.a * 10000 + vector::length(&first.data) * 1000
            + second.a * 10 + (*vector::borrow(&second.data, 0) as u64)
    }
}

module 0x42::gnf {
    use std::vector;

    struct R has key { xs: vector<u64>, ys: vector<u64> }

    struct S has copy, drop { xs: vector<u64>, n: u64 }

    enum E has drop { V { xs: vector<u64>, n: u64 }, W { ys: vector<u64> } }

    public fun resource(account: signer, addr: address, value: u64): u64 acquires R {
        move_to(&account, R { xs: vector[], ys: vector[value] });
        0x0::test_utils::force_gc();
        let stored = borrow_global<R>(addr);
        vector::length(&stored.xs) * 1000 + *vector::borrow(&stored.ys, 0)
    }

    public fun enum_local(base: u64): u64 {
        let variant_v = E::V { xs: vector[], n: base };
        let variant_w = E::W { ys: vector[] };
        0x0::test_utils::force_gc();
        let from_v = match (&variant_v) { E::V { xs, n: count } => vector::length(xs) * 1000 + *count, E::W { ys: _ } => 0 };
        let from_w = match (&variant_w) { E::W { ys } => vector::length(ys), E::V { xs: _, n: _ } => 99 };
        from_v * 10 + from_w
    }

    public fun vector_of_structs(base: u64): u64 {
        let items = vector[S { xs: vector[], n: base }, S { xs: vector[base], n: base + 1 }, S { xs: vector[], n: base + 2 }];
        0x0::test_utils::force_gc();
        let total = 0;
        let index = 0;
        while (index < vector::length(&items)) {
            let item = vector::borrow(&items, index);
            total = total * 100 + vector::length(&item.xs) * 10 + item.n;
            index = index + 1;
        };
        total
    }
}

// RUN: execute 0x42::gnf::resource --args 0x42, 0x42, 9
// CHECK: results: 9
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::gnf::enum_local --args 5
// CHECK: results: 50
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::gnf::vector_of_structs --args 3
// CHECK: results: 31405
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::event::emit_empty_then_gc --args 4
// CHECK: results: 2040057 | events: module 0x1::event::MyEvent 0x040000000000000000; module 0x1::event::MyEvent 0x05000000000000000107
// CHECK-GC-COUNT: 1
