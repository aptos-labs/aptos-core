module 0x815::m {
    fun t2(u1: &mut u64, u2: &mut u64): (&mut u64, &mut u64) {
        (u1, u2)
    }
    fun test_4() {
        let _x: &u64;
        let _y: &u64;
        (_x, _y) = t2(&mut 3, &mut 4);
    }

    fun test_5() {
        let (_x, _y): (&u64, &u64) = t2(&mut 3, &mut 4);
    }

    fun foo(x: &mut u64): &u64 {
        *x = 2;
        x
    }

    fun bar(x: &u64): &u64 {
        x
    }

    public fun foobar(x: u64): u64 {
        *bar(foo(&mut x))
    }
}
