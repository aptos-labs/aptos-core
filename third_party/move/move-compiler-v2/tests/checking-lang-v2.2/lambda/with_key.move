module 0xc0ffee::m {
    fun caller(f: ||u64 has key): u64 {
        f()
    }

    public fun test1(): u64 {
        let f: ||u64 has key = || 1;
        f()
    }

    public fun test2(): u64 {
        let f: ||u64 has key = || 1;
        caller(f)
    }

    struct S has key {
        i: u64,
    }

    fun apply(f: ||S has key): S {
        f()
    }

    public fun test3(): S {
        let s = S { i: 1 };
        let f: ||S has key = || s;
        apply(f)
    }
}

module 0xc0ffee::n {
    fun wrap<T>(x: T, y: T, z: T): vector<T> {
        vector[x, y, z]
    }

    public fun test(): vector<||u64 has copy> {
        let f1: || u64 has copy = || 1;
        let f2: || u64 has copy + key = || 2;
        let f3: || u64 has copy + key = || 3;
        let v: vector<||u64 has copy> = wrap<||u64 has copy>(f2, f1, f3);
        v
    }
}
