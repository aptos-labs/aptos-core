//# publish
module 0xc0ffee::m {
    fun copy_kill(_p: u64): u64 {
        let a = _p;
        let b = a;
        _p = _p + 1;
        b + a
    }

    public fun main() {
        assert!(copy_kill(10) == 20, 0);
    }
}

//# run 0xc0ffee::m::main
