module 0x42::borrow {

    struct S {
        f: u64
    }

    fun mut_field(s: &S): u64 {
        let r = &mut s.f;
        *r = 22;
        *r
    }

    fun mut_expr(x: u64): u64 {
        let r = &mut (x + 1);
        *r = 22;
        *r
    }
}
