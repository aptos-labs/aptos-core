// Differential test for `vector::move_range`.
//
// The test stdlib's `vector` module doesn't declare `move_range`, so it is
// declared here under a test-only module name and registered on both VMs (see
// `v1_test_natives.rs` and `make_all_test_natives`). Each case builds
// `from`/`to`, splices a range across them, and returns the resulting vectors
// so both VMs can be compared. `vector<u8>` cases use byte literals (loaded
// from the constant pool); the `u64` case builds via `push_back` and reads
// elements back to confirm the per-element byte math.

// RUN: publish
module 0x1::vector_natives {
    public native fun move_range<T>(
        from: &mut vector<T>,
        removal_position: u64,
        length: u64,
        to: &mut vector<T>,
        insert_position: u64,
    );
}
module 0x42::vec_move_range {
    use std::vector;
    use 0x1::vector_natives::move_range;

    // length == 1 fast path.
    public fun single_element(): (vector<u8>, vector<u8>) {
        let from = b"\x01\x02\x03\x04\x05";
        let to = b"\x0a\x14\x1e";
        move_range(&mut from, 1, 1, &mut to, 1);
        (from, to)
    }

    // Full drain of `from` appended to the end of `to`.
    public fun full_drain_append(): (vector<u8>, vector<u8>) {
        let from = b"\x01\x02\x03";
        let to = b"\x09";
        move_range(&mut from, 0, 3, &mut to, 1);
        (from, to)
    }

    // Tail of `from` appended to the end of `to`.
    public fun tail_split_append(): (vector<u8>, vector<u8>) {
        let from = b"\x01\x02\x03\x04";
        let to = b"\x09";
        move_range(&mut from, 2, 2, &mut to, 1);
        (from, to)
    }

    // General splice: insert in the middle of `to`, forcing `to` to grow.
    public fun general_splice(): (vector<u8>, vector<u8>) {
        let from = b"\x01\x02\x03\x04\x05";
        let to = b"\x0a\x14\x1e";
        move_range(&mut from, 1, 2, &mut to, 1);
        (from, to)
    }

    // Destination starts empty (null), exercising the allocate-from-donor path.
    public fun empty_destination(): (vector<u8>, vector<u8>) {
        let from = b"\x07\x08";
        let to = vector::empty<u8>();
        move_range(&mut from, 0, 2, &mut to, 0);
        (from, to)
    }

    // 8-byte elements: build, splice, then read elements back.
    public fun u64_elements(): (u64, u64, u64, u64) {
        let from = vector::empty<u64>();
        vector::push_back(&mut from, 100);
        vector::push_back(&mut from, 200);
        vector::push_back(&mut from, 300);
        vector::push_back(&mut from, 400);
        let to = vector::empty<u64>();
        vector::push_back(&mut to, 10);
        vector::push_back(&mut to, 20);
        move_range(&mut from, 1, 2, &mut to, 1);
        // to = [10, 200, 300, 20], from = [100, 400]
        (
            vector::length(&to),
            *vector::borrow(&to, 1),
            *vector::borrow(&to, 2),
            *vector::borrow(&from, 1),
        )
    }

    // removal_position + length exceeds from's length.
    public fun removal_out_of_bounds(): vector<u8> {
        let from = b"\x01\x02";
        let to = b"\x09";
        move_range(&mut from, 1, 5, &mut to, 0);
        to
    }

    // insert_position exceeds to's length.
    public fun insert_out_of_bounds(): vector<u8> {
        let from = b"\x01\x02\x03";
        let to = b"\x09";
        move_range(&mut from, 0, 1, &mut to, 5);
        to
    }
}

// RUN: execute 0x42::vec_move_range::single_element
// CHECK: results: 0x01030405, 0x0a02141e

// RUN: execute 0x42::vec_move_range::full_drain_append
// CHECK: results: 0x, 0x09010203

// RUN: execute 0x42::vec_move_range::tail_split_append
// CHECK: results: 0x0102, 0x090304

// RUN: execute 0x42::vec_move_range::general_splice
// CHECK: results: 0x010405, 0x0a0203141e

// RUN: execute 0x42::vec_move_range::empty_destination
// CHECK: results: 0x, 0x0708

// RUN: execute 0x42::vec_move_range::u64_elements
// CHECK: results: 4, 200, 300, 400

// RUN: execute 0x42::vec_move_range::removal_out_of_bounds
// CHECK: aborted: code 1

// RUN: execute 0x42::vec_move_range::insert_out_of_bounds
// CHECK: aborted: code 1
