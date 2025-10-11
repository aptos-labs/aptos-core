module 0xc0ffee::m {
    //Shadowing tests.

    public fun test1_warn(): u64{
        let ret: u64 = 1;
        {
            let ret = 6;
            {
                ret += 1;
                ret *= 2;
                ret
            };
        };
        ret
    }

    public fun test2_no_warn(): u64{
        let ret: u64 = 1;
        {
            let ret2 = &mut ret;
            {
                let ret = 6;
                *ret2 += {
                    ret += 1;
                    ret *= 2;
                    ret
                };
            };
        };
        ret
    }
}
