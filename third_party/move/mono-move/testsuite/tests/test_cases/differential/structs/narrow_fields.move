// RUN: publish
module 0x1::test {
    struct Mixed has copy, drop {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
    }

    fun get_a(a: u8, b: u16, c: u32, d: u64): u8 {
        let s = Mixed { a, b, c, d };
        s.a
    }
    fun get_b(a: u8, b: u16, c: u32, d: u64): u16 {
        let s = Mixed { a, b, c, d };
        s.b
    }
    fun get_c(a: u8, b: u16, c: u32, d: u64): u32 {
        let s = Mixed { a, b, c, d };
        s.c
    }
    fun get_d(a: u8, b: u16, c: u32, d: u64): u64 {
        let s = Mixed { a, b, c, d };
        s.d
    }
}

// Distinct per-field values confirm each field is read from its own
// (correctly padded) offset rather than a neighbor's.
// RUN: execute 0x1::test::get_a --args 200, 40000, 3000000000, 18000000000000000000
// CHECK: results: 200
// RUN: execute 0x1::test::get_b --args 200, 40000, 3000000000, 18000000000000000000
// CHECK: results: 40000
// RUN: execute 0x1::test::get_c --args 200, 40000, 3000000000, 18000000000000000000
// CHECK: results: 3000000000
// RUN: execute 0x1::test::get_d --args 200, 40000, 3000000000, 18000000000000000000
// CHECK: results: 18000000000000000000
