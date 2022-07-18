module 0xA550C18::HelloWorld {
    use std::ascii::{Self, String};
    use std::signer;
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;

    struct ModuleData has key, store {
        global_counter: u64,
        state: String,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 0, state: ascii::string(b"init") }
        );
    }

    public fun foo(addr: address): u64 {
        Coin::balance<TestCoin>(addr)
    }

    public entry fun hi(sender: &signer, msg: String) acquires ModuleData {
        borrow_global_mut<ModuleData>(signer::address_of(sender)).state = msg;
    }
}
