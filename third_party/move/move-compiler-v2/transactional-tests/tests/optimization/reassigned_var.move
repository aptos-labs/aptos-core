//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let _a = 1;
        let b = 2;
        _a = 9; // reassigned
        _a + b
    }

    public fun main() {
        assert!(test() == 11, 0);
    }
}

//# run 0xc0ffee::m::main
