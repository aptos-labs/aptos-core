// RUN: publish --print(micro-ops)
module 0x42::generic_enum_basic {
    enum Either<L, R> has drop {
        Left { l: L },
        Right { r: R },
    }

    fun left_u64(v: u64): u64 {
        let e = Either::Left<u64, bool> { l: v };
        match (e) {
            Either::Left { l } => l,
            Either::Right { r: _ } => 0,
        }
    }

    fun pick_variant(sel: u64): u64 {
        let e: Either<u64, bool> = if (sel == 0) {
            Either::Right { r: true }
        } else {
            Either::Left { l: sel }
        };
        match (e) {
            Either::Left { l } => l,
            Either::Right { r } => if (r) { 1000 } else { 2000 },
        }
    }

    fun swapped_right_u64(v: u64): u64 {
        let e = Either::Right<bool, u64> { r: v };
        match (e) {
            Either::Left { l } => if (l) { 1 } else { 2 },
            Either::Right { r } => r,
        }
    }

    fun test_is_left(sel: u64): u64 {
        let e: Either<u64, bool> = if (sel == 0) {
            Either::Left { l: 5 }
        } else {
            Either::Right { r: false }
        };
        if (e is Either::Left) { 1 } else { 0 }
    }
}

// RUN: execute 0x42::generic_enum_basic::left_u64 --args 42
// CHECK: results: 42

// RUN: execute 0x42::generic_enum_basic::pick_variant --args 0
// CHECK: results: 1000

// RUN: execute 0x42::generic_enum_basic::pick_variant --args 77
// CHECK: results: 77

// RUN: execute 0x42::generic_enum_basic::swapped_right_u64 --args 314159
// CHECK: results: 314159

// RUN: execute 0x42::generic_enum_basic::test_is_left --args 0
// CHECK: results: 1

// RUN: execute 0x42::generic_enum_basic::test_is_left --args 1
// CHECK: results: 0
