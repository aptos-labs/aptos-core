// RUN: publish --print(stackless)
module 0x42::copy_prop_allowed {
    use std::vector;

    struct Pair has copy, drop { a: u64, b: u64 }

    fun make(seed: u64): vector<u64> {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, seed);
        vector::push_back(&mut items, seed + 1);
        items
    }

    fun bump(x: u64): u64 { x + 1 }

    fun pair_sum(p: Pair): u64 { p.a + p.b }

    fun consume_len(items: vector<u64>): u64 { vector::length(&items) }

    // Copy of a scalar: the copy and the original are interchangeable bits;
    // propagation may collapse the copy.
    fun scalar_copy(v: u64): u64 {
        let d = copy v;
        bump(d) + v
    }

    // Move of a vector: dst and src name the same object; the move chain may
    // collapse even though the type owns heap storage.
    fun vector_move(seed: u64): u64 {
        let v = make(seed);
        let d = v;
        consume_len(d)
    }

    // Copy of a pointer-free local struct: still propagatable.
    fun struct_copy(seed: u64): u64 {
        let p = Pair { a: seed, b: 2 };
        let d = copy p;
        pair_sum(d) + p.a
    }
}

// RUN: execute 0x42::copy_prop_allowed::scalar_copy --args 5
// CHECK: results: 11

// RUN: execute 0x42::copy_prop_allowed::vector_move --args 5
// CHECK: results: 2

// RUN: execute 0x42::copy_prop_allowed::struct_copy --args 5
// CHECK: results: 12
