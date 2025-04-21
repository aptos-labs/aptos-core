module 0x42::test {
    struct S {} has copy;

    struct T<T> { x: T, y: S } has drop;
}
