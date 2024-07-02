//# publish
module 0x42::M {
    fun t2(u1: &mut u64, u2: &mut u64): (&mut u64, &mut u64) {
        (u1, u2)
    }
    public fun bar() {
        let x: &u64;
        let y: &u64;
        (x, y) = t2(&mut 3, &mut 4);
        assert!(*x == 3, 1);
        assert!(*y == 4, 1);
    }

    public fun bar2() {
        let x: &u64;
        let y: &mut u64;
        (x, y) = t2(&mut 3, &mut 4);
        assert!(*x == 3, 1);
        assert!(*y == 4, 1);
    }

    struct S has drop {
       a: u64
    }

    fun t4(u1: &mut S, u2: &mut S): (&mut S, &mut S) {
        (u1, u2)
    }
    public fun bar3() {
        let x: &S;
        let y: &mut S;
        let g = S {
           a: 2
        };
        let h = S {
            a: 3
        };
        (x, y) = t4(&mut g, &mut h);
        assert!(x.a == 2, 1);
        assert!(y.a == 3, 1);
    }

    public fun bar4() {
        let x: &S;
        let y: &S;
        let g = S {
            a: 2
        };
        let h = S {
            a: 3
        };
        (x, y) = t4(&mut g, &mut h);
        assert!(x.a == 2, 1);
        assert!(y.a == 3, 1);
    }

}

//# run 0x42::M::bar

//# run 0x42::M::bar2

//# run 0x42::M::bar3

//# run 0x42::M::bar4
