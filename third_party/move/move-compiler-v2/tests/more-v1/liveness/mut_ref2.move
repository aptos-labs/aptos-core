module 0x42::m {

    struct R has key {
        v: u64
    }

    fun some(_r: &mut R) {}

    fun some2(_r: &mut R, _t: &mut R) {}

    fun id(r: &mut R): &mut R { r }

    fun f1_ok() {
        let r = R { v: 0 };
        let x = &mut r;
        // expected ok since x is used and assigned again
        some(x);
        some(x);
    }

    fun f1a_ok() {
        let r = R { v: 0 };
        let x = &mut r;
        *x; // Expected ok because x is only read; ability analysis will check whether read is ok
        some(x);
        some(x);
    }

    fun f1b_ok() {
        let r = R { v: 0 };
        let x = &mut r;
        some(x);
        *x; // Same as f1aok
        some(x);
    }


    fun f3_ok() {
        let r = R { v: 0 };
        let x = &mut r;
        some(x); // expected ok
        x = &mut r;
        some(x);
    }

    fun f4_ok() {
        let r = R { v: 0 };
        let x = &mut r;
        x = id(x); // expected ok
        some(x);
    }
}
