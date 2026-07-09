// RUN: publish
module 0x42::copy_xfer_gc {
    enum Boxed has copy, drop { V { a: u64, b: u64 } }

    // Takes the enum by value, so the caller must materialize an independent
    // copy into this argument slot.
    fun fold_in(acc: u64, boxed: Boxed): u64 {
        match (boxed) {
            V { a, b } => (acc ^ a) + (b << 1),
        }
    }

    public fun churn(n: u64): u64 {
        // `live` is read on every iteration, so each `fold_in(_, live)` copies
        // it into the argument slot rather than moving it.
        let live = Boxed::V { a: 7, b: 11 };
        let digest: u64 = 0;
        let i = 0;
        while (i < n) {
            digest = fold_in(digest, live);
            // Discard a fresh heap-backed enum as garbage to keep the heap full,
            // so the next deep-copy allocation collects it mid-copy. Its
            // contents are never read; it exists only as allocation pressure.
            let _garbage = Boxed::V { a: i, b: i };
            i = i + 1;
        };
        // `live` must still be intact after all those collections.
        fold_in(digest, live)
    }
}

// RUN: execute 0x42::copy_xfer_gc::churn --args 200 --heap-size 96
// CHECK: results: 4829
// CHECK-GC-COUNT: 198
