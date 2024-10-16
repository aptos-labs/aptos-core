//# publish
module 0xc0ffee::m {
    public fun test(a: u64): u64 {
        let _x = a + a;
        let y = 2;
        y
    }

    public fun main() {
        assert!(test(5) == 2, 0);
    }
}

//# run 0xc0ffee::m::main
