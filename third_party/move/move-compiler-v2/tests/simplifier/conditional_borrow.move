//# publish --print-bytecode
module 0x8675::M {
    fun test1(r: u64): u64 {  // 7 or 2
        let x = 3;
        let tref = &mut { if (r < 4) { r } else { x } }; // &mut tmp = 7 or 3
        *tref = 10; // ignored, writes to *&tmp
        let y = r;  // 7 or 2
        let tref2 = &mut y;
        *tref2 = *tref2 + 1;  // y = 8 or 3
        let z = y;  // 8 or 3
        let tref3 = &mut (z + 0);
        *tref3 = *tref3 + 2; // ignored, writes to temp
        let a = z;  // 8 or 3
        let tref4 = &mut { let _q = 1; a };
        *tref4 = *tref4 + 4; // ignored, writes to temp
        let tref5 = &mut { a };
        *tref5 = *tref5 + 8; // ignored, writes to temp
        let tref6 = &mut { 3; a };
        *tref6 = *tref6 + 16; // ignored, writes to temp
        a // 8 or 3
    }
    public fun test(): u64 {
        test1(7) + test1(2) // 11
    }

    struct S has copy, drop {
        f: u64
    }

    fun test1b(r: S): u64 {  // 7 or 2
        let x = S { f: 3 };
        let tref = &mut { if (r.f < 4) { r } else { x } }; // &mut tmp = 7 or 3
        (*tref).f = 10; // ignored, writes to *&tmp
        let y = r;  // 7 or 2
        let tref2 = &mut y;
        (*tref2).f = (*tref2).f + 1;  // y = 8 or 3
        let z = y;  // 8 or 3
        let tref3 = &mut z.f;
        (*tref3) = (*tref3) + 1; // ignored, writes to temp
        let a = z;  // 8 or 3
        let tref4 = &mut { let _q = 1; a.f };
        (*tref4) = (*tref4) + 1; // ignored, writes to temp
        let tref5 = &mut { a.f };
        *tref5 = *tref5 + 8; // ignored, writes to temp
        let tref6 = &mut { 3; a.f };
        *tref6 = *tref6 + 16; // ignored, writes to temp
        a.f // 8 or 3
    }
    public fun testb(): u64 {
        test1b(S{ f: 7 }) + test1b(S{ f: 2 }) // 11
    }
}

//# run 0x8675::M::test

//# run 0x8675::M::testb
