module 0x42::test {
    struct S0();

    struct S1(u8);

    struct S2(u8, bool);

    struct Unit {}

    enum E {
        V1,
        V2 {},
        V3(),

    }

    fun arity_mismatch_0(): S1 {
        S1()
    }

    fun arity_mismatch1(): S1 {
        S1(0, 1)
    }

    fun arity_mismatch2() {
        S2();
        S2(1);
        S2(1, false, 2);
    }

    fun type_mismatch() {
        S0(false);
        S1(@0x42);
        S2((), 1);
    }

    fun unit_constructor() {
        Unit();
        E::V1();
        E::V2();
        E::V3 {};
    }
}
