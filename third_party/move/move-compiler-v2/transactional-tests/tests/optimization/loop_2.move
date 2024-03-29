//# publish
module 0xc0ffee::m {

    fun test(p: u64): u64 {
        let a = p;
        let count = 0;
        while (count < 10) {
            a = p;
            count = count + 1;
        };
        a // copy `a := p` should be available
    }

    public fun main() {
        assert!(test(42) == 42, 5);
    }
}

//# run 0xc0ffee::m::main
