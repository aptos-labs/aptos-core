module 0x66::m {

    enum E {
        A(u64),
        B,
        C(bool)
    }

    fun variant_test(e: &E): bool {
        match (e) {
            A(_) => true,
            _ => false
        }
    }
}
