//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let _t = 1;
        let u = 2;
        _t = _t + u;
        let b = u;
        b + u
    }

    public fun main() {
        assert!(test() == 4, 0);
    }
}

//# run 0xc0ffee::m::main
