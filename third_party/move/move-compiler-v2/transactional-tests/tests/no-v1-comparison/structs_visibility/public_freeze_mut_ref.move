//# publish
module 0x42::freeze_mut_ref {

    public struct G has drop { f: u64 }

}

//# publish
module 0x42::test_freeze_mut_ref {
    use 0x42::freeze_mut_ref::G;

    fun t1(s: &mut G): &G {
        s
    }

    public fun t5(s: &mut G): (u64, u64, u64) {
        let x = 0;
        let f = { x = x + 1; &mut ({x = x + 1; s}).f };
        let y = &mut 2;
        let z: &u64;

        *({*f = 0; z = y; f}) = 2;
        (*z, *f, x)
    }


    fun test_1() {
        let x: &u64 = &mut 0;
        assert!(*x == 0, 0);
        let g = G {f: 3};
        let y = t1(&mut g);
        assert!(y.f == 3, 1);
    }

    fun test_2() {
        let para_g = G { f: 50 };
        let (z, f, x) = t5(&mut para_g);
        assert!(z == 2, 0);
        assert!(f == 2, 1);
        assert!(x == 2, 2);
    }


    fun test_6() {
        let s1 = G {f: 2};
        let s2 = G {f: 3};
        let x;
        x = if (true) &s1 else &mut s2;
        assert!(x.f == 2, 0);
    }

    fun test_7() {
        let s1 = G {f: 2};
        let s2 = G {f: 3};
        let x: &G = if (true) &s1 else &mut s2;
        assert!(x.f == 2, 0);
    }

}

//# run 0x42::test_freeze_mut_ref::test_1

//# run 0x42::test_freeze_mut_ref::test_2

//# run 0x42::test_freeze_mut_ref::test_6

//# run 0x42::test_freeze_mut_ref::test_7
