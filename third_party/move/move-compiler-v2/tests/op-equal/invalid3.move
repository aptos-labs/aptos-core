module 0x42::test {
    fun test0() {
        1 += 1;
    }

    fun foo(): u8 {
        42
    }

    fun test1() {
        foo() += 1;
    }
}
