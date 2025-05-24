//# publish
module 0x42::Test {
    fun f3() {
        let ff: || has drop = || {};

        (|| {
            *(&mut {ff}) = || {};
        })();

        let ff2: || has copy+drop = || {};
        ff2();
    }

}

//# run --verbose 0x42::Test::f3
