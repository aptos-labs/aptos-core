// Copying a struct with two vector fields where one or both were never pushed to. The copy
// is then written and both owners are read back.

// RUN: publish
module 0x66::copy_empty {
    use std::vector;

    struct Two has copy, drop { a: vector<u64>, b: vector<u64> }

    fun junk(value: u64): u64 {
        let garbage = vector[value, value, value, value];
        vector::length(&garbage)
    }

    fun code(two: &Two): u64 {
        let digest = vector::length(&two.a) * 1000000 + vector::length(&two.b) * 10000;
        if (!vector::is_empty(&two.a)) digest = digest + *vector::borrow(&two.a, 0) * 100;
        if (!vector::is_empty(&two.b)) digest = digest + *vector::borrow(&two.b, 0);
        digest
    }

    public fun first_empty(value: u64): (u64, u64) {
        let orig = Two { a: vector[], b: vector[value, value + 1] };
        junk(value);
        let dup = orig;
        *vector::borrow_mut(&mut dup.b, 0) = 9;
        vector::push_back(&mut dup.a, 7);
        (code(&orig), code(&dup))
    }

    // Nothing allocates after the copy: heap sizes 80 and 111 make the copy of `b` collect,
    // and 112 needs no collection.
    public fun first_empty_overwrite(value: u64): (u64, u64) {
        let orig = Two { a: vector[], b: vector[value, value + 1] };
        junk(value);
        let dup = orig;
        *vector::borrow_mut(&mut dup.b, 0) = 9;
        (code(&orig), code(&dup))
    }

    public fun second_empty(value: u64): (u64, u64) {
        let orig = Two { a: vector[value, value + 1], b: vector[] };
        junk(value);
        let dup = orig;
        *vector::borrow_mut(&mut dup.a, 0) = 9;
        vector::push_back(&mut dup.b, 7);
        (code(&orig), code(&dup))
    }

    public fun both_empty(value: u64): (u64, u64) {
        let orig = Two { a: vector[], b: vector[] };
        junk(value);
        let dup = orig;
        vector::push_back(&mut dup.b, value);
        (code(&orig), code(&dup))
    }
}

// RUN: execute 0x66::copy_empty::first_empty --args 5
// CHECK: results: 20005, 1020709

// RUN: execute 0x66::copy_empty::second_empty --args 5
// CHECK: results: 2000500, 2010907

// RUN: execute 0x66::copy_empty::both_empty --args 5
// CHECK: results: 0, 10005

// RUN: execute 0x66::copy_empty::first_empty_overwrite --args 5 --heap-size 80
// CHECK: results: 20005, 20009
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::copy_empty::first_empty_overwrite --args 5 --heap-size 111
// CHECK: results: 20005, 20009
// CHECK-GC-COUNT: 1

// RUN: execute 0x66::copy_empty::first_empty_overwrite --args 5 --heap-size 112
// CHECK: results: 20005, 20009
// CHECK-GC-COUNT: 0
