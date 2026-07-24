// RUN: publish
module 0x66::move_range {
    use std::vector;

    // Sentinel-prefixed digit-pair encoding (each element < 100), so element
    // order and leading values survive readback. `which`: 0 selects the source
    // vector, 1 the destination.
    fun encode(items: &vector<u64>): u64 {
        let total = 1;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 100 + *vector::borrow(items, pos);
            pos = pos + 1;
        };
        total
    }

    public fun move_u64(removal_position: u64, length: u64, insert_position: u64, which: u64): u64 {
        let from = vector[3, 4, 5, 6];
        let to = vector[1, 2];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        if (which == 0) encode(&from) else encode(&to)
    }

    // `to` starts empty (a null-pointer vector), forcing a fresh allocation.
    public fun move_into_empty(removal_position: u64, length: u64, which: u64): u64 {
        let from = vector[7, 8, 9];
        let to = vector::empty<u64>();
        vector::move_range(&mut from, removal_position, length, &mut to, 0);
        if (which == 0) encode(&from) else encode(&to)
    }

    // 1-byte elements exercise an element size below the pointer width.
    fun encode_bytes(items: &vector<u8>): u64 {
        let total = 1;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 100 + (*vector::borrow(items, pos) as u64);
            pos = pos + 1;
        };
        total
    }

    public fun move_bytes(removal_position: u64, length: u64, insert_position: u64, which: u64): u64 {
        let from = vector[1u8, 2u8, 3u8, 4u8];
        let to = vector[9u8];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        if (which == 0) encode_bytes(&from) else encode_bytes(&to)
    }

    // 32-byte elements exercise a wide element size.
    public fun move_wide(removal_position: u64, length: u64, insert_position: u64, which: u64): u256 {
        let from = vector[100u256, 200u256, 300u256];
        let to = vector[900u256];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        let items = if (which == 0) &from else &to;
        let total = 1u256;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 1000 + *vector::borrow(items, pos);
            pos = pos + 1;
        };
        total
    }

    // Elements that themselves hold heap pointers (single-element inner
    // vectors), with a GC forced after the move to check the moved elements are
    // traced under their new owner and the vacated slots are not.
    fun encode_nested(items: &vector<vector<u64>>): u64 {
        let total = 1;
        let pos = 0;
        while (pos < vector::length(items)) {
            total = total * 100 + *vector::borrow(vector::borrow(items, pos), 0);
            pos = pos + 1;
        };
        total
    }

    public fun move_nested(removal_position: u64, length: u64, insert_position: u64, which: u64): u64 {
        let from = vector[vector[1], vector[2], vector[3]];
        let to = vector[vector[9]];
        vector::move_range(&mut from, removal_position, length, &mut to, insert_position);
        0x0::test_utils::force_gc();
        if (which == 0) encode_nested(&from) else encode_nested(&to)
    }
}

// Legacy parity: [3,4,5,6] move 2 from pos 1 into [1,2] at pos 1.
// from -> [3,6]
// RUN: execute 0x66::move_range::move_u64 --args 1, 2, 1, 0
// CHECK: results: 10306
// to -> [1,4,5,2]
// RUN: execute 0x66::move_range::move_u64 --args 1, 2, 1, 1
// CHECK: results: 101040502

// Move the whole source to the end of `to`, growing `to` (cap 2 -> 6).
// from -> []
// RUN: execute 0x66::move_range::move_u64 --args 0, 4, 2, 0
// CHECK: results: 1
// to -> [1,2,3,4,5,6]
// RUN: execute 0x66::move_range::move_u64 --args 0, 4, 2, 1
// CHECK: results: 1010203040506

// Single element to the front (the length-1 fast path).
// from -> [4,5,6]
// RUN: execute 0x66::move_range::move_u64 --args 0, 1, 0, 0
// CHECK: results: 1040506
// to -> [3,1,2]
// RUN: execute 0x66::move_range::move_u64 --args 0, 1, 0, 1
// CHECK: results: 1030102

// Zero-length move is a no-op (but still bounds-checked).
// from -> [3,4,5,6]
// RUN: execute 0x66::move_range::move_u64 --args 2, 0, 1, 0
// CHECK: results: 103040506
// to -> [1,2]
// RUN: execute 0x66::move_range::move_u64 --args 2, 0, 1, 1
// CHECK: results: 10102

// removal_position + length past the end of `from`. The legacy native aborts;
// the micro-op surfaces a runtime error, like the other vector ops.
// RUN: execute 0x66::move_range::move_u64 --args 1, 5, 0, 0
// CHECK-V1-SUBSTR: aborted: code 1
// CHECK-V2-SUBSTR: VecMoveRange index out of bounds

// insert_position past the end of `to`.
// RUN: execute 0x66::move_range::move_u64 --args 0, 1, 5, 0
// CHECK-V1-SUBSTR: aborted: code 1
// CHECK-V2-SUBSTR: VecMoveRange index out of bounds

// Move into an empty (null-pointer) destination.
// from -> [9]
// RUN: execute 0x66::move_range::move_into_empty --args 0, 2, 0
// CHECK: results: 109
// to -> [7,8]
// RUN: execute 0x66::move_range::move_into_empty --args 0, 2, 1
// CHECK: results: 10708

// 1-byte elements: [1,2,3,4] move 2 from pos 1 into [9] at pos 1.
// from -> [1,4]
// RUN: execute 0x66::move_range::move_bytes --args 1, 2, 1, 0
// CHECK: results: 10104
// to -> [9,2,3]
// RUN: execute 0x66::move_range::move_bytes --args 1, 2, 1, 1
// CHECK: results: 1090203

// 32-byte elements: [100,200,300] move 2 from pos 0 into [900] at pos 1.
// from -> [300]
// RUN: execute 0x66::move_range::move_wide --args 0, 2, 1, 0
// CHECK: results: 1300
// to -> [900,100,200]
// RUN: execute 0x66::move_range::move_wide --args 0, 2, 1, 1
// CHECK: results: 1900100200

// Pointer-holding elements with a GC after the move: [[1],[2],[3]] move 2 from
// pos 0 into [[9]] at pos 1.
// from -> [[3]]
// RUN: execute 0x66::move_range::move_nested --args 0, 2, 1, 0
// CHECK: results: 103
// to -> [[9],[1],[2]]
// RUN: execute 0x66::move_range::move_nested --args 0, 2, 1, 1
// CHECK: results: 1090102
