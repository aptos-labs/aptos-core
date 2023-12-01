module 0x42::m {

    struct R has key {
        v: u64
    }

    fun some(_r: R) {}

    fun f1_ok() {
        let r = R { v: 0 };
        let x = move r;
        some(x);
        some(x);
    }

    fun f1_fail() {
        let r = R { v: 0 };
        let x = move r; // expected to fail
        some(x);
        some(r);
    }
}
