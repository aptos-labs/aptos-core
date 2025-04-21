module 0x815::M {

    fun captures_ref(x: u64): u64 {
        let r = &x;
        let f = || *r + 1;
        f()
    }

    fun captures_ref_curry(x: u64): u64 {
        let r = &x;
        let f = |y| consume_ref(r, y);
        f(2)
    }

    fun captures_mut_ref(x: u64): u64 {
        let r = &mut x;
        let f = || *r + 1;
        f()
    }

    fun consume_ref(r: &u64, x: u64): u64 {
        *r + x
    }
}
