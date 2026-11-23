// Enum variants with vector fields: replacing them through a `match &mut` binding, growing
// a vector inside a variant, copying a variant with nested vectors and then changing the
// original, and a vector of generic `Opt<vector<u64>>` values, with collections in between.

// RUN: publish
module 0x42::vpf {
    use std::vector;

    struct Bag has copy, drop { items: vector<vector<u64>> }
    enum E has copy, drop {
        Empty,
        Data { pre: u8, v: vector<u64>, bag: Bag },
    }
    enum Opt<T> has copy, drop { None, Some { t: T } }

    fun total(value: &E): u64 {
        match (value) {
            E::Empty => 0,
            E::Data { pre, v: values, bag } => {
                let sum = (*pre as u64);
                vector::for_each_ref(values, |elem| sum = sum + *elem);
                vector::for_each_ref(&bag.items, |inner| vector::for_each_ref(inner, |elem| sum = sum + *elem));
                sum
            },
        }
    }

    public fun replace_pointer_field(count: u64): u64 {
        let value = E::Data { pre: 1, v: vector[1], bag: Bag { items: vector[] } };
        let index = 0;
        while (index < count) {
            match (&mut value) {
                E::Data { pre: _, v: values, bag } => {
                    *values = vector[index, index];
                    *bag = Bag { items: vector[vector[index], vector[1, 2]] };
                },
                E::Empty => (),
            };
            0x0::test_utils::force_gc();
            index = index + 1;
        };
        total(&value)
    }

    public fun grow_in_payload(count: u64): u64 {
        let value = E::Data { pre: 0, v: vector[], bag: Bag { items: vector[vector[]] } };
        let index = 0;
        while (index < count) {
            match (&mut value) {
                E::Data { pre: _, v: values, bag } => {
                    vector::push_back(values, index);
                    vector::push_back(vector::borrow_mut(&mut bag.items, 0), 1);
                },
                E::Empty => (),
            };
            index = index + 1;
        };
        total(&value)
    }

    public fun deep_copy(): (u64, u64) {
        let src = E::Data { pre: 2, v: vector[3, 4], bag: Bag { items: vector[vector[5], vector[6, 7]] } };
        let dup = src;
        match (&mut src) {
            E::Data { pre, v: values, bag } => {
                *pre = 0;
                vector::push_back(values, 100);
                vector::push_back(vector::borrow_mut(&mut bag.items, 1), 1000);
            },
            E::Empty => (),
        };
        0x0::test_utils::force_gc();
        let junk = vector[vector[9u64]];
        vector::push_back(&mut junk, vector[8]);
        (total(&src), total(&dup))
    }

    public fun generic_pointer_payload(count: u64): u64 {
        let opts: vector<Opt<vector<u64>>> = vector[];
        let index = 0;
        while (index < count) {
            vector::push_back(&mut opts, Opt::Some { t: vector[index, index + 1] });
            vector::push_back(&mut opts, Opt::None);
            0x0::test_utils::force_gc();
            index = index + 1;
        };
        let sum = 0;
        vector::for_each_ref(&opts, |opt: &Opt<vector<u64>>| sum = sum + match (opt) {
            Opt::Some { t: values } => vector::length(values) + *vector::borrow(values, 1),
            Opt::None => 1,
        });
        sum
    }
}

// RUN: execute 0x42::vpf::replace_pointer_field --args 5 --heap-size 512
// CHECK: results: 16
// CHECK-GC-COUNT: 5

// RUN: execute 0x42::vpf::grow_in_payload --args 40 --heap-size 1536
// CHECK: results: 820
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::vpf::deep_copy --heap-size 512
// CHECK: results: 1125, 27

// RUN: execute 0x42::vpf::generic_pointer_payload --args 6 --heap-size 2048
// CHECK: results: 39
// CHECK-GC-COUNT: 6
