module 0xA550C18::HelloWorld {
    use AptosFramework::Signer;
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;
    use AptosFramework::ASCII::{Self, String};

    struct ModuleData has key, store {
        global_counter: u64,
        state: String,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 0, state: ASCII::string(b"init") }
        );
    }

    public fun foo(addr: address): u64 {
        Coin::balance<TestCoin>(addr)
    }

    public(script) fun hi(sender: &signer, msg: String) acquires ModuleData {
        borrow_global_mut<ModuleData>(Signer::address_of(sender)).state = msg;
    }
}
