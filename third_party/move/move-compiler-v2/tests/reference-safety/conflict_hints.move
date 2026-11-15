// Hints report only borrows that conflict with the failed check.
module 0x42::m {
    struct S has drop {
        f: u64,
        g: u64,
    }

    // Freezing only conflicts with mutable borrows, so `imm` is not reported.
    fun freeze_while_mutably_borrowed(s: &mut S): u64 {
        let imm = &s.f;
        let mutable = &mut s.g;
        let frozen: &S = s;
        *mutable = 1;
        *imm + frozen.f
    }

    // Writing conflicts with any borrow, so `imm` is reported.
    fun write_while_borrowed(s: &mut S): u64 {
        let imm = &s.f;
        *s = S { f: 1, g: 2 };
        *imm
    }

    // The `&s.f` diagnostic reports `same` and omits `other`, which borrows the disjoint field `g`.
    fun borrow_field_while_field_mutably_borrowed(s: &mut S): u64 {
        let other = &mut s.g;
        let same = &mut s.f;
        let read = &s.f;
        *same = 1;
        *other = 2;
        *read
    }
}
