module 0xc0ffee::m {
    public fun test1(x: u64, z: u64) {
        let y = &mut x;
        *y = z;
    }

    public fun test2(x: u64) {
        let a = &mut x;
        *a = 2;
        let b = &mut x;
        *b = 3;
    }

}
