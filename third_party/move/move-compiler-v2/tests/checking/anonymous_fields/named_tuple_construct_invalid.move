module 0x42::test {
    struct S(u8);

    fun arity_mismatch0(): S {
        S()
    }

    fun arity_mismatch1(): S {
        S(0, 1)
    }

    fun type_mismatch(): S {
        S(false)
    }
}
