//# publish
module 0xc0ffee::m {
    fun id(x: u64): u64 {
        x
    }

    fun test(p: u64): u64 {
        let _a = p;
        let b = p;
        let c = b;
        id(id(id(c)))
    }

    public fun run() {
        assert!(test(50) == 50, 0);
    }
}

//# run 0xc0ffee::m::run
