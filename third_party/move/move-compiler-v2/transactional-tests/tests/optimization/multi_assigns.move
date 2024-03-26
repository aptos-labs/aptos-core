//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let _x = 1;
        _x = 2;
        _x
    }

    public fun main() {
        assert!(test() == 2, 0);
    }
}

//# run 0xc0ffee::m::main
