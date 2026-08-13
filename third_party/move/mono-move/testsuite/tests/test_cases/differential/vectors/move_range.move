// RUN: publish
module 0x66::move_range {
    use std::vector;

    public fun move_u64(removal_position: u64, length: u64, insert_position: u64): (vector<u64>, vector<u64>) {
        let from = vector[3, 4, 5, 6];
        let to = vector[1, 2];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        (from, to)
    }

    public fun move_into_empty(removal_position: u64, length: u64): (vector<u64>, vector<u64>) {
        let from = vector[7, 8, 9];
        let to = vector::empty<u64>();
        vector::move_range(&mut from, removal_position, length, &mut to, 0);
        (from, to)
    }

    public fun move_bytes(removal_position: u64, length: u64, insert_position: u64): (vector<u8>, vector<u8>) {
        let from = vector[1u8, 2u8, 3u8, 4u8];
        let to = vector[9u8];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        (from, to)
    }

    // u256 vectors aren't renderable by the harness, so fold each into a scalar.
    fun encode_wide(items: &vector<u256>): u256 {
        let total = 1u256;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 1000 + *vector::borrow(items, pos);
            pos = pos + 1;
        };
        total
    }

    public fun move_wide(removal_position: u64, length: u64, insert_position: u64): (u256, u256) {
        let from = vector[100u256, 200u256, 300u256];
        let to = vector[900u256];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        (encode_wide(&from), encode_wide(&to))
    }

    // Inner vectors hold heap pointers; `move_nested` forces a GC after the move
    // to check the moved elements are traced under their new owner.
    fun encode_nested(items: &vector<vector<u64>>): u64 {
        let total = 1;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 100 + *vector::borrow(vector::borrow(items, pos), 0);
            pos = pos + 1;
        };
        total
    }

    public fun move_nested(removal_position: u64, length: u64, insert_position: u64): (u64, u64) {
        let from = vector[vector[1], vector[2], vector[3]];
        let to = vector[vector[9]];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        0x0::test_utils::force_gc();
        (encode_nested(&from), encode_nested(&to))
    }
}

// Results are (from, to) after the move.
// RUN: execute 0x66::move_range::move_u64 --args 1, 2, 1
// CHECK: results: [3, 6], [1, 4, 5, 2]

// Moving the whole source grows `to`.
// RUN: execute 0x66::move_range::move_u64 --args 0, 4, 2
// CHECK: results: [], [1, 2, 3, 4, 5, 6]

// RUN: execute 0x66::move_range::move_u64 --args 0, 1, 0
// CHECK: results: [4, 5, 6], [3, 1, 2]

// Zero-length move is a no-op but still bounds-checked.
// RUN: execute 0x66::move_range::move_u64 --args 2, 0, 1
// CHECK: results: [3, 4, 5, 6], [1, 2]

// Out-of-bounds range, then out-of-bounds insert position; both abort.
// RUN: execute 0x66::move_range::move_u64 --args 1, 5, 0
// CHECK: aborted: code 1 in 0x1::vector
// RUN: execute 0x66::move_range::move_u64 --args 0, 1, 5
// CHECK: aborted: code 1 in 0x1::vector

// Empty (null-pointer) destination.
// RUN: execute 0x66::move_range::move_into_empty --args 0, 2
// CHECK: results: [9], [7, 8]

// RUN: execute 0x66::move_range::move_bytes --args 1, 2, 1
// CHECK: results: 0x0104, 0x090203

// RUN: execute 0x66::move_range::move_wide --args 0, 2, 1
// CHECK: results: 1300, 1900100200

// RUN: execute 0x66::move_range::move_nested --args 0, 2, 1
// CHECK: results: 103, 1090102
