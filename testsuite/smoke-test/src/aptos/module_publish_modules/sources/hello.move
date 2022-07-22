module 0xA550C18::HelloWorldForModulePublish {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;

    struct ModuleData has key, store {
        global_counter: u64,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 0 }
        );
    }

    public fun foo(addr: address): u64 {
        coin::balance<AptosCoin>(addr)
    }
}
