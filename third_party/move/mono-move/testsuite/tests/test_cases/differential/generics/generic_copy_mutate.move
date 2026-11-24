// Values holding vectors, copied inside a generic function, are independent of their source
// and of each other: element overwrites, pushes, variant changes and nested pops
// through one copy are not seen through the others, read back after a collection. In
// `closure_box` one copy's function value is replaced; the source and the other copy keep
// the original closure and its captured vector.

// RUN: publish
module 0x42::gcm {
    use std::vector;

    struct Box<T> has copy, drop { t: T }
    enum Either<L, R> has copy, drop { Left { l: L }, Right { r: R } }

    public fun add(lhs: u64, rhs: u64): u64 { lhs + rhs }

    fun dup<T: copy>(value: &T): (T, T) { (*value, *value) }

    fun pair(values: &vector<u64>): u64 { *vector::borrow(values, 0) * 1000 + *vector::borrow(values, 1) }

    public fun box_vec(): (u64, u64, u64, u64) {
        let src = Box { t: vector[1u64, 2] };
        let (copy_a, copy_b) = dup(&src);
        *vector::borrow_mut(&mut copy_a.t, 0) = 100;
        *vector::borrow_mut(&mut copy_b.t, 1) = 200;
        0x0::test_utils::force_gc();
        vector::push_back(&mut copy_a.t, 3);
        (pair(&src.t), pair(&copy_a.t), pair(&copy_b.t),
         vector::length(&copy_a.t) * 100 + vector::length(&src.t) * 10 + vector::length(&copy_b.t))
    }

    public fun either(): (u64, u64) {
        let src: Either<vector<u64>, u8> = Either::Left { l: vector[1, 2] };
        let (copy_a, copy_b) = dup(&src);
        match (&mut copy_a) {
            Either::Left { l: values } => {
                *vector::borrow_mut(values, 0) = 50;
                vector::push_back(values, 3);
            },
            Either::Right { r: value } => *value = 0,
        };
        copy_b = Either::Right { r: 7 };
        0x0::test_utils::force_gc();
        let la = match (&copy_a) { Either::Left { l: values } => vector::length(values) * 100 + *vector::borrow(values, 0), Either::Right { r: value } => (*value as u64) };
        let lb = match (&copy_b) { Either::Left { l: values } => vector::length(values), Either::Right { r: value } => (*value as u64) };
        let ls = match (&src) { Either::Left { l: values } => vector::length(values) * 100 + *vector::borrow(values, 0), Either::Right { r: value } => (*value as u64) };
        (la * 1000 + ls, lb)
    }

    public fun weigh(values: vector<u64>, arg: u64): u64 { *vector::borrow(&values, 1) * 10 + arg }

    public fun closure_box(arg: u64): (u64, u64) {
        let captured = vector[7u64, 9];
        let src: Box<|u64|u64 has copy + drop> = Box { t: |value| weigh(captured, value) };
        let (copy_a, copy_b) = dup(&src);
        copy_a.t = |value| add(value, 100);
        0x0::test_utils::force_gc();
        ((copy_a.t)(arg) * 1000 + (src.t)(arg), (copy_b.t)(arg))
    }

    fun sum(boxes: &Box<vector<Box<vector<u64>>>>): u64 {
        let total = 0;
        vector::for_each_ref(&boxes.t, |inner| total = total * 10 + vector::length(&inner.t));
        total
    }

    // Copies a vector of structs that each hold a vector.
    public fun nested(): (u64, u64) {
        let src = Box { t: vector[Box { t: vector[1u64] }, Box { t: vector[2u64, 3] }] };
        let (copy_a, copy_b) = dup(&src);
        vector::push_back(&mut vector::borrow_mut(&mut copy_a.t, 1).t, 4);
        vector::pop_back(&mut vector::borrow_mut(&mut copy_b.t, 0).t);
        *vector::borrow_mut(&mut vector::borrow_mut(&mut copy_b.t, 1).t, 0) = 8;
        0x0::test_utils::force_gc();
        let first = *vector::borrow(&vector::borrow(&src.t, 1).t, 0) * 10
            + *vector::borrow(&vector::borrow(&copy_b.t, 1).t, 0);
        (sum(&copy_a) * 1000 + sum(&src), sum(&copy_b) * 100 + first)
    }
}

// RUN: execute 0x42::gcm::box_vec --heap-size 512
// CHECK: results: 1002, 100002, 1200, 322
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::gcm::either --heap-size 512
// CHECK: results: 350201, 7
// CHECK-GC-COUNT: 1

// 560 bytes is the smallest heap in which the three closure copies made after the forced
// collection fit, so that collection is the only one.
// RUN: execute 0x42::gcm::closure_box --args 5 --heap-size 560
// CHECK: results: 105095, 95
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::gcm::nested --heap-size 512
// CHECK: results: 13012, 228
// CHECK-GC-COUNT: 1
