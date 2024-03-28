//# publish
module 0xc0ffee::m {
    public fun test1(_x: u64) {
        _x = _x;
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

    public fun main() {
        assert!(test2(5) == 5, 0);
        assert!(test3() == 1, 1);
        assert!(test4(55) == 55, 2);
    }

}

//# run 0xc0ffee::m::main
