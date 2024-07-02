//# publish
module 0xc0ffee::m {

    fun test() {
        let x = 0;
        let _r1 = &x;
        let y = 3;
        let z = &mut y;
        *z = x;
        _r1;
    }

    fun test2() {
        let x = 0;
        let _r1 = &x;
        let _v = vector[x];
        _r1;
    }

}
