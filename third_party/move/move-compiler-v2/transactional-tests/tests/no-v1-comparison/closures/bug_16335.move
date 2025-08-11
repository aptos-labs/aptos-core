//# publish
module 0x42::Test {
    fun f(x: &mut || has drop) {
        *x = || {};
    }

    fun f2() {
        let x: || has copy+drop = ||{};
        f(&mut x);
    }
}

//# run --verbose 0x42::Test::f2
