module 0xA550C18::HelloWorld {
    use std::string::{Self, String};
    use std::signer;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;

    struct ModuleData has key, store {
        global_counter: u64,
        state: String,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 0, state: string::utf8(b"init") }
        );
    }

    public fun foo(addr: address): u64 {
        coin::balance<AptosCoin>(addr)
    }

    public entry fun hi(sender: &signer, msg: String) acquires ModuleData {
        borrow_global_mut<ModuleData>(signer::address_of(sender)).state = msg;
    }
}
