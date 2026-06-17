// RUN: publish --print(micro-ops,frame-layout)
module 0x66::vec_pack_micro_ops {
    use std::vector;

    public fun pack_u64(first: u64, second: u64, third: u64): u64 {
        let items = vector[first, second, third];
        *vector::borrow(&items, 1)
    }

    public fun pack_nested(low: u8, high: u8): u8 {
        let front = vector[low, low];
        let back = vector[high];
        let outer = vector[front, back];
        *vector::borrow(vector::borrow(&outer, 0), 1)
            + *vector::borrow(vector::borrow(&outer, 1), 0)
    }

    public fun swap_pair(first: u64, second: u64): u64 {
        let items = vector[first, second];
        vector::swap(&mut items, 0, 1);
        *vector::borrow(&items, 0)
    }

    public fun consume_empty(value: u64): u64 {
        let items = vector::empty<u64>();
        vector::destroy_empty(items);
        value
    }

    fun take_two(bytes: vector<u8>, nums: vector<u64>): u64 {
        vector::length(&bytes) * 10 + vector::length(&nums)
    }

    public fun literal_args(byte_val: u8, num_val: u64): u64 {
        take_two(vector[byte_val], vector[num_val, num_val])
    }
}

// RUN: execute 0x66::vec_pack_micro_ops::pack_u64 --args 5, 6, 7
// CHECK: results: 6

// RUN: execute 0x66::vec_pack_micro_ops::pack_nested --args 3, 4
// CHECK: results: 7

// RUN: execute 0x66::vec_pack_micro_ops::swap_pair --args 10, 20
// CHECK: results: 20

// RUN: execute 0x66::vec_pack_micro_ops::consume_empty --args 99
// CHECK: results: 99

// RUN: execute 0x66::vec_pack_micro_ops::literal_args --args 8, 250
// CHECK: results: 12
