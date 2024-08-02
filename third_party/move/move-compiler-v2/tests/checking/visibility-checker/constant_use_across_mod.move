module 0xdeadbeef::M {
    const FIVE: u64 = 5;

    public fun five(): u64 {
        FIVE
    }
}

module 0xdeadbeef::N {
    use 0xdeadbeef::M;

    public fun five(): u64 {
        M::FIVE
    }

    public fun another_five(): u64 {
        use 0xdeadbeef::M::FIVE;
        FIVE
    }
}
