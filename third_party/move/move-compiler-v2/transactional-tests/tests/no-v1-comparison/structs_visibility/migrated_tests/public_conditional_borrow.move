//# publish
module 0x8675::M {
    public struct S has copy, drop {
        f: u64
    }
}

//# publish
module 0x8675::test_M {
    use 0x8675::M::S;
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

//# run 0x8675::test_M::testb
