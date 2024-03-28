//# publish
module 0xc0ffee::m {
    fun test(p: u64): bool {
        let _a = p;
        let b = _a;
        let c = b;

        _a = p + 1; // kill b := a
        b == c
    }

    public fun main() {
        assert!(test(100) == true, 0);
    }

}

//# run 0xc0ffee::m::main
