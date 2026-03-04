module 0xc0ffee::m {
    fun test(x: i8): i8 {
        match (x) {
            -0x1::m::foo() => 1,
            _ => 0,
        }
    }
}
