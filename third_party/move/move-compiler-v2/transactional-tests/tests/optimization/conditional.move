//# publish
module 0xc0ffee::m {
    fun test(p: bool): u64 {
        let x = 2;
        if (p) {
            let y = 3;
            y
        } else {
            let y = x + 1;
            y
        }
    }

    public fun main() {
        assert!(test(true) == 3, 0);
        assert!(test(false) == 3, 1);
    }
}

//# run 0xc0ffee::m::main
