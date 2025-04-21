module 0x42::test {
    struct S0();

    struct S1(u8);

    struct S2(u8, bool);

    struct S3<T>(T, u8);

    struct S4 {}

    struct S5<T> {
        x: T,
        y: u8
    }

    enum E1 {
        V1,
        V2(),
        V3(u8, bool)
    }

    fun S0_inhabited(): S0 {
        S0()
    }

    fun S1_inhabited(): S1 {
        S1(0)
    }

    fun S2_inhabited(): S2 {
        S2(0, false)
    }

    fun S3_test<T>(x: T): S3<T> {
        S3(x, 0)
    }

    fun nested_0(): S3<S4> {
        S3(S4 {}, 0)
    }

    fun nested_1(): S5<S0> {
        S5<S0> {
            x: S0(),
            y: 0
        }
    }

    fun test_variant() {
        E1::V1 {};
        E1::V2();
        E1::V3(42, true);
    }
}
