module publisher::test {
    struct ModuleData has key, store {
        global_counter: u64,
    }

    fun init_module(sender: &signer) {
        move_to(
            sender,
            ModuleData { global_counter: 42 }
        );
    }
}
