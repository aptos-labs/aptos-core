module 0x42::Test {
    fun f() {
        let x : ||(||) = || {||{}};
        *(&mut x) = x();
    }
}
