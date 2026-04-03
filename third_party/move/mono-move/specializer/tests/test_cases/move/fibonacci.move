module 0x42::fibonacci {
    fun fib(n: u64): u64 {
        if (n <= 1) {
            n
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }
}
