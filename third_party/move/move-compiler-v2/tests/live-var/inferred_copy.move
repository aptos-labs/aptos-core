module 0x42::m {

    struct R has key, copy {
        v: u64
    }

    fun some(_r: R) {}

    fun some2(_r: R, _r1: R) {}

    fun id(r: R): R { r }

    fun f1_ok() {
        let r = R { v: 0 };
        some(r);
        some(r);
    }

    fun f2_ok() {
        let r = R { v: 0 };
        some2(r, r);
    }

    fun f3_ok() {
        let r = R { v: 0 };
        let y = r;
        some(r);
        some(y);
    }

    fun f4_fail() {
        let r = R { v: 0 };
        let y = move r;
        some(r);
        some(y);
    }
}
