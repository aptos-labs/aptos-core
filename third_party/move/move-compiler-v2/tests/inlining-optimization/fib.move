module 0xc0ffee::m {
    fun fib(n: u64): u64 {
        if (n < 2) {
            n
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }
}
