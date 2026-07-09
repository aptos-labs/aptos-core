// RUN: publish
module 0x42::generic_mixed_nesting {
    struct Box<T> has copy, drop {
        value: T,
    }

    enum Either<L, R> has copy, drop {
        Left { l: L },
        Right { r: R },
    }

    // Struct containing a generic enum.
    struct Holder<T> has copy, drop {
        e: Either<T, u64>,
    }

    fun make_left<T>(v: T): Holder<T> {
        Holder { e: Either::Left { l: v } }
    }

    fun read_left(v: u64): u64 {
        let h = make_left(v);
        match (h.e) {
            Either::Left { l } => l,
            Either::Right { r } => r,
        }
    }

    // Enum containing a generic struct.
    fun boxed_right(v: u64): u64 {
        let e: Either<bool, Box<u64>> = Either::Right { r: Box { value: v } };
        match (e) {
            Either::Left { l: _ } => 0,
            Either::Right { r } => r.value,
        }
    }

    // Both nesting directions inside one generic function.
    fun roundtrip<T: copy + drop>(v: T): Either<bool, Box<T>> {
        let h = Holder { e: Either::Left { l: Box { value: v } } };
        match (h.e) {
            Either::Left { l } => Either::Right { r: l },
            Either::Right { r: _ } => Either::Left { l: false },
        }
    }

    fun run_roundtrip(v: u64): u64 {
        match (roundtrip(v)) {
            Either::Left { l: _ } => 0,
            Either::Right { r } => r.value,
        }
    }
}

// RUN: execute 0x42::generic_mixed_nesting::read_left --args 31
// CHECK: results: 31

// RUN: execute 0x42::generic_mixed_nesting::boxed_right --args 64
// CHECK: results: 64

// RUN: execute 0x42::generic_mixed_nesting::run_roundtrip --args 99
// CHECK: results: 99
