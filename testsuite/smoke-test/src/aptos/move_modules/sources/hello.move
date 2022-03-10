module 0xA550C18::HelloWorld {
    use AptosFramework::TestCoin;

    public fun foo(addr: address): u64 {
        TestCoin::balance_of(addr)
    }
}
