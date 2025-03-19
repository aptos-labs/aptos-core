//# publish
module 0x42::test {

    fun if_then(c: bool, t: u64, f: u64): u64 {
        if (c) t else f
    }

    public fun run(): bool {
        let curry_001 = |x, y| if_then(true, x, y);
        let curry_010 = |c, x| if_then(c, 10, x);
        let curry_011 = |x| if_then(false, 10, x);
        let curry_100 = |c, x| if_then(c, x, 20);

        assert!(curry_001(3, 4)  == 3);
        assert!(curry_010(true, 22222)  == 10);
        assert!(curry_011(43) == 43);
        assert!(curry_100(false, 23333) == 20);
        true
    }

}

//# run 0x42::test::run
