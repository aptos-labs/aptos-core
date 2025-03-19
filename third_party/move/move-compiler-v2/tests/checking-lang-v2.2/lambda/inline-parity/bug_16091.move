module 0x42::Test {
    struct S<T1> has drop, store {
        t1: T1,
    }

    inline fun foo<T1>(self: &S<T1>, f:|&S<T1>|) {
        f(self)
    }

    struct T has key, drop {
        t: S<u64>
    }


    public fun test() {
        let t = S {
            t1: 2 as u64,
        };
        t.foo(|t2| {
            t2.foo(|_t4| {
                // do nothing
            });
        });
    }
}
