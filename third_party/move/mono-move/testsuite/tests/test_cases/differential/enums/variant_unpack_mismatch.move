// RUN: publish
module 0x42::enum_unpack_mismatch {
    enum E has drop {
        A { x: u64 },
        B { y: u64 },
    }

    // Irrefutably destructure as B.
    fun unpack_as_b(sel: u64): u64 {
        let e = if (sel == 0) { E::A { x: 5 } } else { E::B { y: 9 } };
        let E::B { y } = e;
        y
    }
}

// RUN: execute 0x42::enum_unpack_mismatch::unpack_as_b --args 0
// CHECK-SUBSTR: STRUCT_VARIANT_MISMATCH

// RUN: execute 0x42::enum_unpack_mismatch::unpack_as_b --args 1
// CHECK: results: 9
