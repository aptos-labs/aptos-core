// Two references to different elements of one vector, live across collections: the caller
// holds one to element 0 while a callee collects twice (allocating a decoy vector in
// between) and only then reads its own reference to element 2. The caller reads element 0
// after the call. The golden pins the lowering and frame layout.

// RUN: publish --print(micro-ops,frame-layout)
module 0x42::two_refs {
    use std::vector;

    fun read_after_gcs(elem: &u64, seed: u64): (u64, u64) {
        0x0::test_utils::force_gc();
        let decoy = vector[seed, seed + 1, seed + 2];
        0x0::test_utils::force_gc();
        (*elem, *vector::borrow(&decoy, 2))
    }

    fun bump_after_gc(elem: &mut u64): u64 {
        0x0::test_utils::force_gc();
        *elem = *elem + 1;
        *elem
    }

    public fun shared(elem0: u64, elem1: u64, elem2: u64): (u64, u64) {
        let values = vector[elem0, elem1, elem2];
        let first = vector::borrow(&values, 0);
        let (third, decoy) = read_after_gcs(vector::borrow(&values, 2), elem0 + 7000);
        (*first, third + decoy * 1000)
    }

    // The callee writes through its reference; the caller then reads the same element
    // and a neighbor through the vector.
    public fun write_through(elem0: u64, elem1: u64, elem2: u64): (u64, u64) {
        let values = vector[elem0, elem1, elem2];
        let elem = vector::borrow_mut(&mut values, 1);
        let seen = bump_after_gc(elem);
        (seen, *vector::borrow(&values, 1) * 100 + *vector::borrow(&values, 2))
    }
}

// RUN: execute 0x42::two_refs::shared --args 11, 22, 33
// CHECK: results: 11, 7013033
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::two_refs::write_through --args 11, 22, 33
// CHECK: results: 23, 2333
// CHECK-GC-COUNT: 1
