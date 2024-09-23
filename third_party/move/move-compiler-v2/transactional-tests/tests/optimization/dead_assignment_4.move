//# publish
module 0xc0ffee::m {
    public fun test1(): u64 {
        let _x = 1;
        let y = 3;
        y
    }

    public fun test2(y: u64): u64 {
        let _x = y;
        y
    }

    public fun test3(y: u64): u64 {
        let _x = y;
        8
    }

    public fun test4(_y: u64): u64 {
        let x = 1;
        x
    }

    public fun main() {
        assert!(test1() == 3, 0);
        assert!(test2(5) == 5, 0);
        assert!(test3(5) == 8, 0);
        assert!(test4(45) == 1, 0);
    }

}

//# run 0xc0ffee::m::main
