module 0xcafe::test1 {
    // implementation
    public fun add_numbers(x: u64, y: u64): u64 {
        x + y - 5 + 3 - x * 2 + 23 - 44 + x * y
    }

    // now we want to benchmark our implementation
    public entry fun benchmark_add_numbers() {
        add_numbers(5,5);
    }
}
