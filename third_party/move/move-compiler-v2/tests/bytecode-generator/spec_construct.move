module 0x42::m {
    use std::vector;
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
}
