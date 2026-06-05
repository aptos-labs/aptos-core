// RUN: publish
module 0x1::test {
    fun deref_u8(r: &u8): u8 { *r }
    fun deref_u16(r: &u16): u16 { *r }
    fun deref_u32(r: &u32): u32 { *r }

    // Borrow the 2nd narrow param (lands at frame offset 1 / 2 / 4, in the tail
    // of an 8-byte-padded frame) and pass the reference through a call.
    fun pass_ref_u8(a: u8, b: u8): u8 { deref_u8(&b) }
    fun pass_ref_u16(a: u16, b: u16): u16 { deref_u16(&b) }
    fun pass_ref_u32(a: u32, b: u32): u32 { deref_u32(&b) }

    // Mutate a narrow tail local through a &mut reference, then read it back.
    fun bump_u8(a: u8, b: u8): u8 {
        let r = &mut b;
        *r = *r + 5;
        b
    }
    fun bump_u16(a: u16, b: u16): u16 {
        let r = &mut b;
        *r = *r + 5;
        b
    }

    struct S has copy, drop { a: u8, b: u16, c: u32 }

    // Borrow a narrow field by reference (the u32 field sits at struct offset
    // 4, i.e. frame tail) and read through the reference.
    fun field_ref_u32(a: u8, b: u16, c: u32): u32 {
        let s = S { a, b, c };
        let r = &s.c;
        *r
    }

    // Write a narrow field through a &mut reference, then read it back.
    fun set_field_u16(a: u8, b: u16, c: u32, nb: u16): u16 {
        let s = S { a, b, c };
        let r = &mut s.b;
        *r = nb;
        s.b
    }
}

// RUN: execute 0x1::test::pass_ref_u8 --args 10, 20
// CHECK: results: 20
// RUN: execute 0x1::test::pass_ref_u16 --args 10, 20
// CHECK: results: 20
// RUN: execute 0x1::test::pass_ref_u32 --args 100, 200
// CHECK: results: 200
// RUN: execute 0x1::test::bump_u8 --args 10, 20
// CHECK: results: 25
// RUN: execute 0x1::test::bump_u16 --args 10, 20
// CHECK: results: 25
// RUN: execute 0x1::test::field_ref_u32 --args 5, 1000, 70000
// CHECK: results: 70000
// RUN: execute 0x1::test::set_field_u16 --args 5, 1000, 70000, 42
// CHECK: results: 42
