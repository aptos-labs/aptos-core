module 0xc0ffee::m {
    fun foo(y: u64, x: &mut u64) {
        *x = y;
    }

    public fun test(): u64 {
        let x = 0;
        foo(3, &mut x);
        foo(4, &mut x);
        x
    }

}
