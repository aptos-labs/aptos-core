// RUN: publish
module 0x42::struct_inline_borrow {
    struct Pair has drop { a: u64, b: u64 }

    public fun inline_borrow(): u64 {
        let p = Pair { a: 10, b: 20 };
        let r = &mut p.b;
        *r = 99;
        p.b
    }
}

// RUN: execute 0x42::struct_inline_borrow::inline_borrow
// CHECK: results: 99
