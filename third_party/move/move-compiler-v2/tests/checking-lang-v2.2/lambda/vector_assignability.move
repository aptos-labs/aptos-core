module 0xc0ffee::m {
    struct NoCopy has drop;

    public fun foo() {
        let x = NoCopy;

        let a: ||u64 has drop = ||{
            let NoCopy = x;
            1
        };

        let b: vector<||u64 has drop + copy> = vector[a];
        (b[0])();
        (b[0])();
    }
}

module 0xc0ffee::n {
    struct NoCopy has drop;

    public fun foo() {
        let x = NoCopy;

        let a: ||u64 has drop = ||{
            let NoCopy = x;
            1
        };

        let b: ||u64 has copy + drop = || 42;

        let v = vector[b];
        (v[0])();
        v[0] = a;
    }
}

module 0xc0ffee::o {
    struct NoCopy has drop;

    fun replace<T>(ref: &mut T, new: T): T {
        abort 0
    }

    public fun foo() {
        let x = NoCopy;

        let a: ||u64 has drop = ||{
            let NoCopy = x;
            1
        };

        let b: ||u64 has copy + drop = || 42;

        let v = vector[b];
        replace(&mut v[0], a);
    }
}

module 0xc0ffee::p {
    public fun foo() {
        let a: ||u64 has copy + drop = || 1;

        let b: ||u64 has drop = || 42;

        let v = vector[b];
        v[0] = a;
        v[0]();
    }
}

module 0xc0ffee::q {
    fun swap<T>(left: &mut T, right: &mut T) {
        abort 0
    }

    public fun foo() {
        let a: ||u64 has copy + drop = || 1;
        let b: ||u64 has drop = || 42;

        swap(&mut a, &mut b);
    }
}
