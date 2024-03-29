//# publish
module 0xc0ffee::m {
    fun test(): u64 {
        let x = 1;
        let y = 2;
        let z = 3;
        x + y + z
    }

    public fun main() {
        assert!(test() == 6, 0);
    }
}

//# run 0xc0ffee::m::main
