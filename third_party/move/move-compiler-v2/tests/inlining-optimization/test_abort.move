module 0xc0ffee::n {
    const ERROR_CODE_2: u64 = 42;

    // not inlined in m
    public fun not_inlined(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE_2);
            *x = 42;
        } else {
            *x = *x + 1;
        }
    }
}


module 0xc0ffee::m {
    const ERROR_CODE: u64 = 42;
    const N_ERROR_CODE_2: u64 = 43;

    fun one(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE);
            *x = 42;
        } else {
            *x = *x + N_ERROR_CODE_2;
        }
    }

    public fun two(): u64 {
        2
    }

    public fun compute(): u64 {
        use 0xc0ffee::n;
        let x = 3;
        // not inlined
        n::not_inlined(&mut x);
        // inlined
        one(&mut x);
        x + two()
    }
}
