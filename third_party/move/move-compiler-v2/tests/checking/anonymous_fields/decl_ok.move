module 0x42::test {
    struct S1();

    struct S2(u8, bool);

    struct S3<T1, T2>(T2, u8, T1);

    enum E1 {
        V1,
        V2(),
        V3(u8, bool)
    }

    fun foo(x : S2) {
        x.0;
        x.1;
    }

    fun bar(x : S2) {
        x.0;
    }

    fun baz() {
        E1::V1 {};
        E1::V2();
        E1::V3(42, true);
    }
}
