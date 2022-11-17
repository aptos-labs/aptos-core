module 0xbeef::UseState {
    use 0xbeef::State;
    use std::signer::address_of;

    public fun version(): u64 {
        0
    }

    public fun state_version(): u64 {
        State::version()
    }

    // init_module
    fun init_module(s: &signer) {
        assert!(@0x0 != address_of(s), 100);
    }
}
