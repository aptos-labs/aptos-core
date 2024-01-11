module 0x42::test {
    inline fun foo(): u32 {
        return 5
    }

    fun test_inline() {
        foo();
    }
}
