// RUN: publish
module 0x42::enum_gc {
    use std::vector;

    enum Shape has drop {
        Circle { r: u64 },
        Square { s: u64 },
    }

    enum Holder has drop {
        Empty,
        List { v: vector<u64> },
    }

    enum Pair has drop {
        One { a: u64 },
        Two { b: u64, c: u64 },
    }

    struct Wrapper has drop {
        x: u64,
        p: Shape,
    }

    // Enum object survives a collection: arm selection checks the tag, the
    // binding checks the field bytes.
    fun enum_survives_gc(): u64 {
        let e = Shape::Circle { r: 42 };
        0x0::test_utils::force_gc();
        match (e) {
            Shape::Circle { r } => r,
            Shape::Square { s } => s + 1000,
        }
    }

    // GC traces the vector owned by the active variant: length and all
    // elements (order-weighted) intact after both objects relocate.
    fun enum_traces_vector(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 10);
        vector::push_back(&mut v, 20);
        vector::push_back(&mut v, 30);
        let h = Holder::List { v };
        0x0::test_utils::force_gc();
        match (h) {
            Holder::Empty => 0,
            Holder::List { v } => {
                vector::length(&v) * 1000
                    + *vector::borrow(&v, 0)
                    + 2 * *vector::borrow(&v, 1)
                    + 3 * *vector::borrow(&v, 2)
            },
        }
    }

    // Variant switch between collections: the first GC relocates the List,
    // the reassignment drops it, the second GC runs with the old vector dead.
    fun variant_switch_across_gc(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 100);
        let h = Holder::List { v };
        0x0::test_utils::force_gc();
        let first = match (&h) {
            Holder::Empty => 0,
            Holder::List { v } => *vector::borrow(v, 0),
        };
        h = Holder::Empty;
        0x0::test_utils::force_gc();
        let is_empty = if (h is Holder::Empty) { 1 } else { 0 };
        first * 10 + is_empty
    }

    // Enum pointer held in a frame-inline struct field is traced as a root.
    fun enum_in_struct_gc(): u64 {
        let w = Wrapper { x: 7, p: Shape::Square { s: 42 } };
        0x0::test_utils::force_gc();
        let p = match (&w.p) {
            Shape::Circle { r } => *r,
            Shape::Square { s } => *s,
        };
        w.x + p
    }

    // Enum pointers inside a heap object (vector elements, one per variant
    // shape) are traced through the vector's descriptor.
    fun enums_in_vector_gc(): u64 {
        let v = vector::empty<Pair>();
        vector::push_back(&mut v, Pair::One { a: 10 });
        vector::push_back(&mut v, Pair::Two { b: 30, c: 40 });
        0x0::test_utils::force_gc();
        let first = match (vector::borrow(&v, 0)) {
            Pair::One { a } => *a,
            Pair::Two { b, c } => *b + *c,
        };
        let second = match (vector::borrow(&v, 1)) {
            Pair::One { a } => *a,
            Pair::Two { b, c } => *b + *c,
        };
        first + second
    }
}

// RUN: execute 0x42::enum_gc::enum_survives_gc
// CHECK: results: 42
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::enum_gc::enum_traces_vector
// CHECK: results: 3140
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::enum_gc::variant_switch_across_gc
// CHECK: results: 1001
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::enum_gc::enum_in_struct_gc
// CHECK: results: 49
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::enum_gc::enums_in_vector_gc
// CHECK: results: 80
// CHECK-GC-COUNT: 1
