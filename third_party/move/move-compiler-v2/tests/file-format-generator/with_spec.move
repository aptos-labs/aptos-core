module 0x42::m {
    use std::vector;
    spec module {
        global var: num;
    }
    struct S {
        data: vector<E>,
    }
    struct E {
        k: u8,
    }
    public fun foo(
        v: &S,
    ): u8 {
        vector::borrow(&v.data, 0).k
    }

    fun bar() {
        spec {
            update var = 1;
        };
    }
}
