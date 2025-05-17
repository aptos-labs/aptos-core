module 0xc0ffee::m {

    inline fun id(f: ||u64 has copy+drop): ||u64 {
        f
    }

    inline fun id_2(f: ||u64 has copy+drop): ||u64 {
        let k = f;
        let y = k();
        || y + f()
    }

    inline fun id_3(f1: ||u64 has copy+drop, x: u64, f2: ||u64 has copy+drop): ||u64 {
        let k = f1;
        let y = k();
        || y + f2() + x
    }

    inline fun id_4(f1: ||u64 has copy+drop, x: u64, f2: ||u64 has copy+drop): ||u64 {
        let k = f2;
        let y = k();
        || y + f1() + x
    }

    public fun test(): u64 {
        id(||1)()
    }

    public fun test_2(): u64 {
        id_2(||1)()
    }

    public fun test_3(): u64 {
        id_3(||1, 2, ||3)()
    }

    public fun test_4(): u64 {
        id_4(||1, 2, ||3)()
    }
}
