//# publish
module 0x42::Test {

    public inline fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun test(): u64 {
        add(1, 2)
    }
}

//# run 0x42::Test::test
