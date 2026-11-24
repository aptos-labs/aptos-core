// `mem::swap` on function values and on empty vectors that were never pushed to. The
// closures capture different values, and each is called after the swap. Swapping two empty
// vectors leaves both unchanged, so those steps push to both afterwards to show they stay
// usable and independent. The empty/non-empty steps (flat and nested) exchange a never-pushed
// vector with a filled one.

// RUN: publish
module 0x42::swap_shapes {
    use std::mem;
    use std::vector;

    public fun closures(): (u64, u64) {
        let offset = 1;
        let factor = 2;
        let left: |u64|u64 has drop = |input| input + offset;
        let right: |u64|u64 has drop = |input| input * factor;
        mem::swap(&mut left, &mut right);
        (left(10), right(10))
    }

    public fun closure_vectors(): (u64, u64) {
        let left: vector<|u64|u64 has copy + drop> = vector[|input| input + 100];
        let right: vector<|u64|u64 has copy + drop> = vector[|input| input + 200, |input| input + 300];
        mem::swap(&mut left, &mut right);
        (left.length() * 1000 + (left[1])(1), right.length() * 1000 + (right[0])(1))
    }

    public fun both_empty(): (vector<u64>, vector<u64>) {
        let left = vector[];
        let right = vector::empty<u64>();
        mem::swap(&mut left, &mut right);
        left.push_back(1);
        right.push_back(2);
        (left, right)
    }

    public fun empty_and_full(): (vector<u64>, vector<u64>) {
        let left = vector[];
        let right = vector[7, 8];
        mem::swap(&mut left, &mut right);
        right.push_back(9);
        (left, right)
    }

    public fun nested_empty_and_full(): (u64, vector<u8>, u64, vector<u8>) {
        let left: vector<vector<u8>> = vector[];
        let right = vector[b"ab", b"c"];
        mem::swap(&mut left, &mut right);
        right.push_back(b"d");
        (left.length(), left[1], right.length(), right[0])
    }

    public fun both_empty_nested(): (u64, vector<u8>, u64, vector<u8>) {
        let left: vector<vector<u8>> = vector[];
        let right: vector<vector<u8>> = vector[];
        mem::swap(&mut left, &mut right);
        left.push_back(b"ab");
        right.push_back(b"c");
        (left.length(), left[0], right.length(), right[0])
    }
}

// RUN: execute 0x42::swap_shapes::closures
// CHECK: results: 20, 11

// RUN: execute 0x42::swap_shapes::closure_vectors
// CHECK: results: 2301, 1101

// RUN: execute 0x42::swap_shapes::both_empty
// CHECK: results: [1], [2]

// RUN: execute 0x42::swap_shapes::empty_and_full
// CHECK: results: [7, 8], [9]

// RUN: execute 0x42::swap_shapes::nested_empty_and_full
// CHECK: results: 2, 0x63, 1, 0x64

// RUN: execute 0x42::swap_shapes::both_empty_nested
// CHECK: results: 1, 0x6162, 1, 0x63
