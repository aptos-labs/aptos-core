// RUN: publish
module 0x42::ld_const_gc_retry {
    const FILLER: vector<u64> = vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    const BLOB: vector<u64> = vector[10, 20, 30, 40, 50];

    // Loads FILLER and returns only its length. Once this frame is popped, no
    // live slot holds FILLER, so it is dead heap until a collection runs.
    fun fill_and_measure(): u64 {
        let filler = FILLER;
        std::vector::length(&filler)
    }

    public fun ld_const_after_gc(with_filler: bool): u64 {
        let filler_len = if (with_filler) fill_and_measure() else 0;
        let blob = BLOB;
        filler_len + std::vector::length(&blob)
    }
}

// FILLER (96 B) + BLOB does not fit a 128 B heap: BLOB's ld_const triggers one GC.
// RUN: execute 0x42::ld_const_gc_retry::ld_const_after_gc --args true --heap-size 128
// CHECK: results: 15
// CHECK-GC-COUNT: 1

// BLOB alone fits a 128 B heap, so no GC runs.
// RUN: execute 0x42::ld_const_gc_retry::ld_const_after_gc --args false --heap-size 128
// CHECK: results: 5
// CHECK-GC-COUNT: 0
