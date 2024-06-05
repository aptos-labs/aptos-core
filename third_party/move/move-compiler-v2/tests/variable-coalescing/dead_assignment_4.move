module 0xc0ffee::m {
    public fun test1(): u64 {
        let x = 1;
        let y = 3;
        y
    }

    public fun test2(y: u64): u64 {
        let x = y;
        y
    }

    public fun test3(y: u64): u64 {
        let x = y;
        8
    }

    public fun test4(y: u64): u64 {
        let x = 1;
        x
    }

}
