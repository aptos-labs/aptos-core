// RUN: publish --print(micro-ops)
module 0x42::enum_by_ref_param {
    enum E has drop {
        A { a: u8, x: u8 },   // x at field index 1 -> data byte offset 1
        B { b: u64, x: u8 },  // x at field index 1 -> data byte offset 8
    }

    // `x` is shared across A and B at the same field index but a different byte
    // offset.
    fun read_x(e: &E): u8 {
        *(&e.x)
    }

    fun read_x_a(): u8 {
        let e = E::A { a: 0, x: 7 };
        read_x(&e)
    }

    fun read_x_b(): u8 {
        let e = E::B { b: 256, x: 9 };
        read_x(&e)
    }
}

// RUN: execute 0x42::enum_by_ref_param::read_x_a
// CHECK: results: 7

// RUN: execute 0x42::enum_by_ref_param::read_x_b
// CHECK: results: 9
