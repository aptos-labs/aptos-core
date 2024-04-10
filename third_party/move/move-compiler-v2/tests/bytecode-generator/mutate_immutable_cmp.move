module 0x8675309::M {
    struct S has drop { f: u64 }
    struct T has drop { s: S }

    fun t0(s: &mut S) {
        *(s: &mut S) = S { f: 2 }; // this is OK
        *(s: &S) = S { f: 0 }; // this is not OK
        *&0 = 1;
        let x = 0;
        let x_ref = &mut x;
        let x_ref: &u64 = x_ref;
        *x_ref = 0;
        let g = S { f: 0};
        let g_ref = &mut g;
        *(g_ref: &S) = S {f : 2};
        let t = T { s: g };
        let t_ref = &mut t;
        let g = S { f: 2};
        (t_ref: &mut T).s = g; // this is OK
        let g = S { f: 3};
        (t_ref: &T).s = g; // this is not OK
    }

    struct G has key, drop {  }

    fun t1() {
        let x: u64 = 3;
        *(&mut x: &u64) = 5;
    }
}
