//# publish
module 0xc0ffee::corner_cases {
    public fun fun1(): u64 {
        let x = 0;
        let r2 = &mut x;
        let r1 = &mut x;
        *r2 = 5;
        *r1
    }

    fun update(x: &mut u64) {
        *x = 5;
    }

    fun get_value(x: &u64): u64 {
        *x
    }

    public fun fun2(): u64 {
        let x = 0;
        let r2 = &mut x;
        let r1 = &mut x;
        update(r2);
        get_value(r1)
    }

    public fun fun3(): u64 {
        let x = 0;
        let r2 = &mut x;
        let r1 = r2;
        *r1 = 5;
        *r2
    }

    public fun fun4(): u64 {
        let x = 0;
        let r2 = &mut x;
        let r1 = &mut x;
        let r3 = &mut x;
        *r2 = 5;
        *r1 + *r3
    }
}

//# run 0xc0ffee::corner_cases::fun1 --verbose

//# run 0xc0ffee::corner_cases::fun2 --verbose

//# run 0xc0ffee::corner_cases::fun3 --verbose

//# run 0xc0ffee::corner_cases::fun4 --verbose
