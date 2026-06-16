// RUN: publish
module 0x42::enum_divergent_offset {
    enum E has drop {
        A { a: u8,  x: u8 },   // x at field index 1 -> data byte offset 1
        B { b: u64, x: u8 },   // x at field index 1 -> data byte offset 8
    }

    // `e.x` groups {A, B} into a single BorrowVariantField (x is at the same
    // field index in both), but x's BYTE offset differs (1 vs 8). The lowering
    // must select the offset by the runtime tag, not assume variants[0]'s.
    fun read_x_a(): u8 {
        let e = E::A { a: 0, x: 7 };
        e.x
    }

    fun read_x_b(): u8 {
        let e = E::B { b: 256, x: 9 };
        e.x
    }
}

// RUN: execute 0x42::enum_divergent_offset::read_x_a
// CHECK: results: 7

// RUN: execute 0x42::enum_divergent_offset::read_x_b
// CHECK: results: 9
