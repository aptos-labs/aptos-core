//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let _x: u32 = 1;
        _x = _x + 1;
        let y: u64 = 2;
        y = y + 1;
        y
    }

    public fun main() {
        assert!(test() == 3, 0);
    }
}

//# run 0xc0ffee::m::main
