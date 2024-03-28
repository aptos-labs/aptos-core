//# publish
module 0xc0ffee::m {
    fun update(p: &mut u64) {
        *p = 0;
    }

    fun test(p: u64): u64 {
        let a = p;
        let b = p;
        let c = b;
        update(&mut a);
        c
    }

    public fun main() {
        assert!(test(5) == 5, 0);
    }
}

//# run 0xc0ffee::m::main
