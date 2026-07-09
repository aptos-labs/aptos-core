// RUN: publish
module 0x66::vec_pack_gc_loop {
    use std::vector;

    fun take_two(bytes: vector<u8>, nums: vector<u64>): u64 {
        vector::length(&bytes) + vector::length(&nums)
    }

    public fun pack_loop(rounds: u64): u64 {
        let total = 0;
        let counter = 0;
        while (counter < rounds) {
            let nested = vector[vector[counter], vector[counter, counter]];
            total = total + vector::length(&nested);
            total = total + take_two(vector[1u8, 2u8], vector[counter, counter, counter]);
            counter = counter + 1;
        };
        total
    }
}

// 7 per round: nested len 2 + take_two (2 + 3).
// RUN: execute 0x66::vec_pack_gc_loop::pack_loop --args 50 --heap-size 512
// CHECK: results: 350
// CHECK-GC-COUNT: 19

// RUN: execute 0x66::vec_pack_gc_loop::pack_loop --args 0 --heap-size 512
// CHECK: results: 0
// CHECK-GC-COUNT: 0
