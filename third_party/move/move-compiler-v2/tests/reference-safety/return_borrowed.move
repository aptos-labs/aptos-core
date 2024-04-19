module 0x42::m {
    use 0x1::vector;

    struct S has copy, drop { f: vector<u64> }

    // The compiler should detect that the returned reference is derived from input parameters.
    fun f(r1: &S, r2: &u64): &u64 {
        let vec = &r1.f;
        if (vector::is_empty(vec)) r2 else vector::borrow(vec, 0)
    }
}
