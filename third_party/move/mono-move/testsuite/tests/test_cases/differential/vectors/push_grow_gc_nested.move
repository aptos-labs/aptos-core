// Vector growth that collects while the vector is an element of another vector, a
// resource field or an enum field, and while the pushed element is itself a vector. After
// setup and one dead `junk` vector, the only allocations are the pushes, so every counted
// collection happens inside a push.

// RUN: publish
module 0x42::pgn {
    use std::vector;

    struct R has key { xs: vector<u64> }

    enum Bag has drop { Empty, Items { xs: vector<u64> } }

    fun sum(values: &vector<u64>): u64 {
        let total = 0;
        let index = 0;
        while (index < vector::length(values)) { total = total + *vector::borrow(values, index); index = index + 1; };
        total
    }

    fun junk(value: u64): u64 {
        let garbage = vector[value, value, value];
        vector::length(&garbage)
    }

    public fun inner_of_outer(count: u64): u64 {
        let outer = vector[vector::empty<u64>(), vector[0u64]];
        junk(count);
        let index = 0;
        while (index < count) {
            vector::push_back(vector::borrow_mut(&mut outer, 1), index);
            index = index + 1;
        };
        sum(vector::borrow(&outer, 1)) + vector::length(vector::borrow(&outer, 0))
    }

    public fun through_resource(account: signer, addr: address, count: u64): u64 {
        move_to(&account, R { xs: vector[] });
        junk(count);
        let holder = borrow_global_mut<R>(addr);
        let index = 0;
        while (index < count) {
            vector::push_back(&mut holder.xs, index);
            index = index + 1;
        };
        sum(&borrow_global<R>(addr).xs)
    }

    public fun through_enum(count: u64): u64 {
        let bag = Bag::Items { xs: vector[] };
        junk(count);
        let index = 0;
        while (index < count) {
            match (&mut bag) {
                Bag::Items { xs } => vector::push_back(xs, index),
                Bag::Empty => {},
            };
            index = index + 1;
        };
        match (&bag) {
            Bag::Items { xs } => sum(xs),
            Bag::Empty => 0,
        }
    }

    // While `outer` grows, the element being pushed is referenced from nowhere else.
    public fun pointer_elem(): u64 {
        let holder = vector[vector[1u64, 2], vector[3, 4], vector[5, 6], vector[7, 8], vector[9, 10]];
        junk(0);
        let outer = vector::empty<vector<u64>>();
        while (!vector::is_empty(&holder)) {
            vector::push_back(&mut outer, vector::pop_back(&mut holder));
        };
        let digest = 0;
        let index = 0;
        while (index < vector::length(&outer)) { digest = digest * 100 + sum(vector::borrow(&outer, index)); index = index + 1; };
        digest
    }
}

// Growing the inner vector to capacity 8 (the last allocation) collects; the grown vector
// must be stored into the outer vector, which the collection moved.
// RUN: execute 0x42::pgn::inner_of_outer --args 5 --heap-size 184
// CHECK: results: 10
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::inner_of_outer --args 5 --heap-size 255
// CHECK: results: 10
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::inner_of_outer --args 5 --heap-size 256
// CHECK: results: 10
// CHECK-GC-COUNT: 0

// The first push allocates the resource's never-pushed field and collects.
// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 4 --heap-size 64
// CHECK: results: 6
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 4 --heap-size 103
// CHECK: results: 6
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 4 --heap-size 104
// CHECK: results: 6
// CHECK-GC-COUNT: 0

// Growing to capacity 16 collects while the vector is a resource field.
// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 9 --heap-size 240
// CHECK: results: 36
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 9 --heap-size 327
// CHECK: results: 36
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_resource --args 0x42, 0x42, 9 --heap-size 328
// CHECK: results: 36
// CHECK-GC-COUNT: 0

// Growing to capacity 16 collects while the vector is an enum field.
// RUN: execute 0x42::pgn::through_enum --args 9 --heap-size 248
// CHECK: results: 36
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_enum --args 9 --heap-size 335
// CHECK: results: 36
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::through_enum --args 9 --heap-size 336
// CHECK: results: 36
// CHECK-GC-COUNT: 0

// Growing `outer` to capacity 8 collects.
// RUN: execute 0x42::pgn::pointer_elem --heap-size 344
// CHECK: results: 1915110703
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::pointer_elem --heap-size 383
// CHECK: results: 1915110703
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::pgn::pointer_elem --heap-size 384
// CHECK: results: 1915110703
// CHECK-GC-COUNT: 0
