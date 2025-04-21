module 0x42::test {
    enum Foo<T> has drop, copy {
        A(T),
        B(u8, bool),
    }

    enum Bar<T> {
        A(T),
        B(u8, bool),
    } has drop, copy;
}
