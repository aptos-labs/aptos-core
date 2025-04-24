//# publish
module 0x42::Test {
    fun f(x: &mut || has drop) {
        *x = || {};
    }

    fun f2() {
        let x: || has copy+drop = ||{};
        f(&mut x);
    }

    fun f3() {
        let ff: || has drop = || {};

        // let ff: || has copy+drop = || {}; // This runs fine

        (|| {
            *(&mut {ff}) = || {};
        })();
    }

}

//# run --verbose 0x42::Test::f2

//# run --verbose 0x42::Test::f3
