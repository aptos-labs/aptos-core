// Closures held in a struct field, an enum field, structs in a vector and a resource field,
// copied and called across collections. In `struct_field` and `enum_field` a helper builds the
// closure from a vector argument, so once it returns only the closure refers to the captured
// vector. In `vector_of_structs` each call checks that the closure still has its own item's
// vector.

// RUN: publish
module 0x42::cif {
    use std::vector;

    struct S has copy, drop { f: |u64|u64 has copy + drop, k: u64 }

    enum Slot has copy, drop { Empty, Full { f: |u64|u64 has copy + drop } }

    struct H has key { f: |u64|u64 has copy + drop + store }

    #[persistent]
    public fun add_k(offset: u64, arg: u64): u64 { arg + offset }

    fun mk(values: vector<u64>, arg: u64): u64 { vector::length(&values) * 100 + arg }

    fun capture(values: vector<u64>): |u64|u64 has copy + drop { |arg| mk(values, arg) }

    // Aborts unless the captured vector's element is `arg`.
    fun check_elem(values: vector<u64>, arg: u64): u64 {
        assert!(*vector::borrow(&values, 0) == arg, 1);
        100 + arg
    }

    public fun struct_field(offset: u64): u64 {
        let original = S { f: capture(vector[1u64, 2, 3]), k: offset };
        0x0::test_utils::force_gc();
        let copied = original;
        0x0::test_utils::force_gc();
        (original.f)(original.k) + (copied.f)(1)
    }

    public fun enum_field(offset: u64): u64 {
        let slot = Slot::Full { f: capture(vector[1u64, 2]) };
        let copy_slot = slot;
        0x0::test_utils::force_gc();
        let first = match (slot) { Slot::Full { f: func } => func(offset), Slot::Empty => 0 };
        0x0::test_utils::force_gc();
        let second = match (copy_slot) { Slot::Full { f: func } => func(offset + 1), Slot::Empty => 0 };
        first * 1000 + second
    }

    public fun vector_of_structs(count: u64): u64 {
        let items = vector::empty<S>();
        let index = 0;
        while (index < count) {
            let values = vector[index];
            vector::push_back(&mut items, S { f: |arg| check_elem(values, arg), k: index });
            index = index + 1;
        };
        0x0::test_utils::force_gc();
        let acc = 0;
        index = 0;
        while (index < count) {
            let item = *vector::borrow(&items, index);
            acc = acc + (item.f)(item.k);
            index = index + 1;
        };
        acc
    }

    public fun store_and_call(account: signer, addr: address, offset: u64): u64 acquires H {
        move_to(&account, H { f: |arg| add_k(offset, arg) });
        0x0::test_utils::force_gc();
        let func = borrow_global<H>(addr).f;
        func(1)
    }
}

// RUN: execute 0x42::cif::struct_field --args 5
// CHECK: results: 606
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::cif::enum_field --args 5
// CHECK: results: 205206
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::cif::vector_of_structs --args 10
// CHECK: results: 1045
// CHECK-GC-COUNT: 1

// With a heap of 1432 bytes, besides the forced collection, the build loop collects once (at
// the last closure's captured data) and each read iteration after the first collects twice,
// once per closure copy.
// RUN: execute 0x42::cif::vector_of_structs --args 10 --heap-size 1432
// CHECK: results: 1045
// CHECK-GC-COUNT: 20

// RUN: execute 0x42::cif::store_and_call --args 0x42, 0x42, 7
// CHECK: results: 8
// CHECK-GC-COUNT: 1
