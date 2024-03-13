module 0x8675::M {
    struct S { f: u64, g: u64 }
    fun id<T>(r: &T): &T {
        r
    }
    fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    inline fun iterloop(x: u64, y: &u64): u64 {
        let r = x + 3;
        while (x > 0) {
            x = x - *y;
        };
        r + x
    }

    fun test1a(x: u64, r: &u64): u64 {
        let t = *r;
        let t3 = iterloop(x, r);
        let t2 = *r + t;
        t2 + t3 + t
    }

    fun test1(r: u64): u64 {
        let t = r;
        let t2 = 0;
        while (r > 0) {
            let x = r;
            r = r - 1;
            t2 = t2 + x;
        };
        let t3 = r + t + t2;
        t3
    }

    fun t0() {
        let v = 0;
        let x = &mut v;
        let y = &mut v; // error in v2
        *x;
        *y;
        if (v == 0) {
            v = 3;
        } else {
            v = 2;
        };
        let q = v;

        let x = id_mut(&mut v);
        let y = &mut v; // error in v2
        *x;
        *y;

        let x = &v;
        let y = &mut v;
        *y;
        *x;
        *y;

        let x = &v;
        let y = &v;
        *x;
        *y;
        *x;

        let x = id(&v);
        let y = &v;
        *x;
        *y;
        *x;
    }
}
