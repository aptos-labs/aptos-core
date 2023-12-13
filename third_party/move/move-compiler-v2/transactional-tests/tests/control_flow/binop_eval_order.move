//# publish
module 0xc0ffee::m {

    public fun test(): u64 {
        let a = 1;
        a + {a = a + 1; a}
    }
}

//# run 0xc0ffee::m::test
