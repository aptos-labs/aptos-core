module 0x42::Test {

    fun apply(f:|u64, u64| u64, x: u64, y: u64): u64 {
        f(x, y)
    }

    fun adder(x: u64, y: u64): u64 {
        x + y
    }

    public fun main(): u64 {
        apply(|x, y| {
            return adder(x, y)
        }, 10, 100)
    }
}
