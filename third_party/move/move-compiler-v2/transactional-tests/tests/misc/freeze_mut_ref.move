//# publish
module 0x42::freeze_mut_ref {
    use std::vector;

    struct G has drop { f: u64 }


    public fun borrow_mut<Element>(
        map: &mut vector<Element>,
    ): &Element {
        vector::borrow_mut(map, 0)
    }

    public fun borrow_mut2<Element>(
        v: &mut Element,
    ): &Element {
        v
    }

    fun t1(s: &mut G): &G {
        s
    }

    fun t2(u1: &mut u64, u2: &mut u64): (&mut u64, &mut u64) {
        (u1, u2)
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


    fun test_3() {
        let vec = vector[0u64];
        let y = borrow_mut(&mut vec);
        assert!(*y == 0, 0);
    }

    // TODO: this case is not handled
    // fun test_4() {
    //     let x: &u64;
    //     let y: &u64;
    //     (x, y) = t2(&mut 3, &mut 4);
    //     assert!(*x == 3, 2);
    //     assert!(*y == 4, 3);
    // }

    fun test_5() {
        let x = 3;
        let a = borrow_mut2(&mut x);
        assert!(*a == 3, 0);
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

//# run 0x42::freeze_mut_ref::test_1

//# run 0x42::freeze_mut_ref::test_2

//# run 0x42::freeze_mut_ref::test_3

//# run 0x42::freeze_mut_ref::test_5

//# run 0x42::freeze_mut_ref::test_6

//# run 0x42::freeze_mut_ref::test_7
