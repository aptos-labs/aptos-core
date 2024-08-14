module 0x42::test {
    struct S() has copy, key;

    struct S1(u8) has drop;

    struct S2<T>(T, u8) has key;

    struct S3<T: key>(T, u8) has key;

    struct S4<T: key> has drop {
        x: u8,
        y: T,
    }

    struct S5<T: copy + key>(T, S3<T>) has key;

    struct S6<phantom T: store>();
}
