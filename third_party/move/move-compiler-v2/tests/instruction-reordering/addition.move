module 0xc0ffee::m {

    public fun test1(x: u64): u64 {
        x + 1
    }

    public fun test2(a: u64, b: u64): u64 {
        let a = a + 1;
        let b = b + 1;
        a + b
    }

    public fun test3(a: u64, b: u64, c: u64, d: u64): u64 {
        let a = 1 + a;
        let b = 1 + b;
        let c = 1 + c;
        let d = 1 + d;
        let _ = a + b;
        c + d
    }

    public fun test4(p: u64, q: u64): u64 {
        let x = p + 1;
        let y = q + 1;
        x + y
    }

    public fun test5(): u64 {
        let x = 1;
        let y = x;
        x = 2;
        let z = x;
        z + y
    }

    public fun test6(): u64 {
        let x = 1;
        let y = x;
        x = 2;
        let z = x;
        y + z
    }

    public fun test7(p: u64, q: u64): u64 {
        let x = p + 1;
        let y = q + 1;
        x + y
    }

    fun one(): u64 {
        1
    }

    public fun test8(x: u64): u64 {
        x + one()
    }

    public fun test9(p: u64, q: u64): u64 {
        let x = &p;
        let y = &q;
        *y + *x
    }

    public fun test10(p: u64, q: u64): u64 {
        let x = &p;
        let y = &q;
        *x + *y
    }
}
