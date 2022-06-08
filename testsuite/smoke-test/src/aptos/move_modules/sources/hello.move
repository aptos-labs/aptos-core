module 0xA550C18::HelloWorld {
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;

    struct ModuleData has key, store {
        global_counter: u64,
    }

    fun init_module(creator: &signer) {
        move_to(
            creator,
            ModuleData { global_counter: 0 }
        );
    }

    public fun foo(addr: address): u64 {
        Coin::balance<TestCoin>(addr)
    }
}
