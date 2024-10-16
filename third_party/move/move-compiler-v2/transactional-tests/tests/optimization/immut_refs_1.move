//# publish
module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = &p;
        let b = a;
        let c = b;
        *c
    }

    public fun main() {
        assert!(test(42) == 42, 0);
    }
}

//# run 0xc0ffee::m::main
