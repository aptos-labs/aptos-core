//# publish
module 0x42::Test {
    fun foo(f:|u64| u64, x: u64): u64 {
        f(x)
    }

    public fun test(): u64 {
        foo(|_| 3, 10)
    }

    public fun main() {
        assert!(test() == 3, 5);
    }
}

//# run 0x42::Test::main
