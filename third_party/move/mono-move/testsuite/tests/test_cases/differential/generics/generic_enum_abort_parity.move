// RUN: publish
module 0x42::generic_enum_abort_parity {
    enum Either<L, R> has drop {
        Left { l: L },
        Right { r: R },
    }

    // Irrefutably destructure as Left: aborts unless the runtime variant
    // matches.
    fun unpack_as_left(sel: u64): u64 {
        let e: Either<u64, bool> = if (sel == 0) {
            Either::Left { l: 7 }
        } else {
            Either::Right { r: true }
        };
        let Either::Left { l } = e;
        l
    }
}

// RUN: execute 0x42::generic_enum_abort_parity::unpack_as_left --args 0
// CHECK: results: 7

// RUN: execute 0x42::generic_enum_abort_parity::unpack_as_left --args 1
// CHECK-SUBSTR: STRUCT_VARIANT_MISMATCH
