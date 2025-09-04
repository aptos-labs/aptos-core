// Example taken from https://github.com/velor-chain/velor-core/issues/12404#issuecomment-2004040746
module 0xc0ffee::m {
    fun test1(x: u64): u64 {
        x + 1
    }

    fun test2(x: u64): u64 {
        x + 2
    }

    public fun test(): u64 {
        test1(2) + test2(5)
    }
}
