module 0x42::test {
    fun foo(a: u8, _b: u8) {
        let x = 0;
        x = x + 1;
        let y = 0;
        let _z = 42;
        // unused call assignment
        let w = bar(false);
    }

    fun bar(x: bool): u8 {
        let y = 0;
        if (x) {
            y = y + 1;
        } else {
            y = y + 2;
        };
        42
    }

    fun baz() {
        let x = 0;
        // shadowing
        let x = 1;
    }
}
