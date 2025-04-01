//# publish
module 0xc0ffee::m {
    public fun test1(): u8 {
        let a = vector[0u8, 128u8];
        a[0]
    }

    public fun test2(): u8 {
        let b = x"0080";
        b[0]
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2
