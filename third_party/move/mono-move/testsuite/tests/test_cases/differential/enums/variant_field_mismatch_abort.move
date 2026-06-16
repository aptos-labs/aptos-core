// RUN: publish
module 0x42::enum_variant_mismatch {
    enum E has drop {
        A { x: u64 },
        B { x: u64, y: u64 },
    }

    // `x` is declared by both variants at the same byte offset: reading it takes
    // the uniform fast path and never aborts.
    fun read_x(sel: u64): u64 {
        let e = if (sel == 0) { E::A { x: 7 } } else { E::B { x: 8, y: 9 } };
        e.x
    }

    // `y` is declared only by B (the divergent-offset path). Reading it on an A
    // (sel 0) must abort; on a B (sel 1) both must return y.
    fun read_y(sel: u64): u64 {
        let e = if (sel == 0) { E::A { x: 7 } } else { E::B { x: 8, y: 9 } };
        e.y
    }
}

// RUN: execute 0x42::enum_variant_mismatch::read_x --args 0
// CHECK: results: 7

// RUN: execute 0x42::enum_variant_mismatch::read_x --args 1
// CHECK: results: 8

// RUN: execute 0x42::enum_variant_mismatch::read_y --args 0
// CHECK-SUBSTR: STRUCT_VARIANT_MISMATCH

// RUN: execute 0x42::enum_variant_mismatch::read_y --args 1
// CHECK: results: 9
