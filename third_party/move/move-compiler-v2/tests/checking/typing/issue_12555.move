module 0xc0ffee::m {
    struct S {
        p: bool,
        x: u64
    }

    public fun foo(testin: bool): u64 {
        let x = S { p: testin, x: 13 };
        let y = S { p: testin, x: 14 };
        let r1 = &mut x;
        let r2 = &mut y;
        *(if (*r1.p) { r1 }
        else { r2 }).x = 15;
        x.x
    }
}
