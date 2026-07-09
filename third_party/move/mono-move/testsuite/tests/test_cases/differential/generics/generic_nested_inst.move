// RUN: publish
module 0x42::generic_nested_inst {
    struct Box<T> has copy, drop {
        value: T,
    }

    struct Pair<A, B> has copy, drop {
        first: A,
        second: B,
    }

    // Nested generic owners: each `.value` resolves against a different
    // instantiated layout.
    fun nested_box(v: u64): u64 {
        let b = Box<Box<u64>> { value: Box { value: v } };
        b.value.value
    }

    fun pair_box(v: u8, w: u128): u128 {
        let p = Pair<Box<u8>, u128> { first: Box { value: v }, second: w };
        (p.first.value as u128) + p.second
    }

    fun pair_of_pairs(x: u64, y: u64): u64 {
        let pp = Pair<Pair<u64, u8>, u64> {
            first: Pair { first: x, second: 3 },
            second: y,
        };
        pp.first.first + (pp.first.second as u64) + pp.second
    }
}

// RUN: execute 0x42::generic_nested_inst::nested_box --args 777
// CHECK: results: 777

// RUN: execute 0x42::generic_nested_inst::pair_box --args 200, 100000000000000000000
// CHECK: results: 100000000000000000200

// RUN: execute 0x42::generic_nested_inst::pair_of_pairs --args 40, 50
// CHECK: results: 93
