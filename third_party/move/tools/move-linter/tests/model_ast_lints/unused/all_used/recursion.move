module 0x42::m {
    // Direct recursion - called by public function
    fun factorial(n: u64): u64 {
        if (n <= 1) 1
        else n * factorial(n - 1)
    }

    // Mutual recursion - both called transitively
    fun is_even(n: u64): bool {
        if (n == 0) true
        else is_odd(n - 1)
    }

    fun is_odd(n: u64): bool {
        if (n == 0) false
        else is_even(n - 1)
    }

    public fun test(): u64 {
        let f = factorial(5);
        if (is_even(f)) f else f + 1
    }
}
