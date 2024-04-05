//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let t = 1;
        let u = 2;
        t = t + 1;
        let b = u;
        b + t
    }

    public fun main() {
        assert!(test() == 4, 6);
    }
}

//# run 0xc0ffee::m::main
