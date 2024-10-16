//# publish
module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let _a = &p;
        let b = p;
        let c = b;
        let d = c;
        d
    }

    public fun main() {
        assert!(test(42) == 42, 5);
    }
}

//# run 0xc0ffee::m::main
