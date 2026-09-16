// Writes through mutable references must be visible through later reads,
// including after freezing a reference or borrowing a struct field.

// RUN: publish
module 0x1::test {
    // Move source cannot form references to references.

    // Swap two locals through plain mutable refs.
    fun swap_via_refs(): (u64, u64) {
        let a = 1;
        let b = 2;
        let ra = &mut a;
        let rb = &mut b;
        let t = *ra;
        *ra = *rb;
        *rb = t;
        (a, b)
    }

    // The frozen reference and the local must both reflect the write.
    fun freeze_then_read(): u64 {
        let x = 10;
        let r = &mut x;
        *r = 20;
        let f = freeze(r);
        *f + x
    }

    // Updating one field through a reference must preserve the other field.
    struct Pair has drop { a: u64, b: u64 }

    fun field_ref_write(): u64 {
        let p = Pair { a: 1, b: 2 };
        let r = &mut p.b;
        *r = *r * 10;
        p.a * 100 + p.b
    }
}

// RUN: execute 0x1::test::swap_via_refs
// CHECK: results: 2, 1

// RUN: execute 0x1::test::freeze_then_read
// CHECK: results: 40

// RUN: execute 0x1::test::field_ref_write
// CHECK: results: 120
