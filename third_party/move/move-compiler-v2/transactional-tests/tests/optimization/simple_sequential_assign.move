//# publish
module 0xc0ffee::m {
    fun sequential(p: u64): u64 {
        let a = p;
        let b = a;
        let c = b;
        let d = c;
        let e = d;
        e
    }

    public fun main() {
        assert!(sequential(50) == 50, 0);
    }
}

//# run 0xc0ffee::m::main
