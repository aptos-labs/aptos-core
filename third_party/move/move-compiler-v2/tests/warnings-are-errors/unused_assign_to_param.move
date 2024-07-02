module 0x42::test {
    fun foo(x: u8) {
        x = 42;
    }

    fun bar(x: u8) {
        if (x > 3) {
            x = 42;
        };
        x = 42;
    }
}
