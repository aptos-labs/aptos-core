//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let _t = 1;
        let u = 2;
        _t = _t + 1;
        let b = u;
        b
    }

    public fun main() {
        assert!(test() == 2, 6);
    }
}

//# run 0xc0ffee::m::main
