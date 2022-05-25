module 0xA550C18::HelloWorld {
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;

    public fun foo(addr: address): u64 {
        Coin::balance<TestCoin>(addr)
    }
}
