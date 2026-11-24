// `vector::move_range` with a removal position past the end of `from`. With a
// zero length and an in-bounds insert position, only the removal-position
// bound can reject the call; the control at removal position == length is a
// valid no-op.

// RUN: publish
module 0x42::move_range_start {
    use std::vector;

    public fun zero_length_at(removal_position: u64): (vector<u64>, vector<u64>) {
        let from = vector[3, 4, 5, 6];
        let to = vector[1, 2];
        vector::move_range(&mut from, removal_position, 0, &mut to, 0);
        (from, to)
    }
}

// RUN: execute 0x42::move_range_start::zero_length_at --args 4
// CHECK: results: [3, 4, 5, 6], [1, 2]

// RUN: execute 0x42::move_range_start::zero_length_at --args 5
// CHECK: aborted: code 1 in 0x1::vector

// RUN: execute 0x42::move_range_start::zero_length_at --args 18446744073709551615
// CHECK: aborted: code 1 in 0x1::vector
