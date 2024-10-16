//# publish
module 0xc0ffee::m {
    fun test(p: u64): bool {
        let a = p;
        let _b = a;
        let c = _b;

        _b = p + 1; // kill b := a, which removes the whole copy chain
        a == c
    }

    public fun main() {
        assert!(test(55), 0);
    }
}

//# run 0xc0ffee::m::main
