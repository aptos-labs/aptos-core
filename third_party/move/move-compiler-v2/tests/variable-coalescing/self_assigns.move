module 0xc0ffee::m {
    public fun test1(x: u64) {
        x = x;
    }

    public fun test2(x: u64): u64 {
        x = x;
        x
    }

    public fun test3(): u64 {
        let i = 0;
        let x = 1;
        while (i < 42) {
            x = x;
            i = i + 1;
        };
        x
    }

    public fun test4(x: u64): u64 {
        let i = 0;
        while (i < 42) {
            x = x;
            i = i + 1;
        };
        x
    }

}
