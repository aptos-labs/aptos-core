//# publish
module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = p;
        let b = &mut p;
        *b = 1;
        a
    }

    public fun main() {
        assert!(test(55) == 55, 0);
    }
}

//# run 0xc0ffee::m::main
