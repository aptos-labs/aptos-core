module 0x42::test {
    struct S {
        x: bool,
        y: u8
    }

    fun simple_0(x: S) {
        let S { .., y } = x;
    }
}
