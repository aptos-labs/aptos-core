module 0xbeef::UseState {
    use 0xbeef::State;

    public fun state_version(): u64 {
        State::version()
    }

    fun init_module(_s: &signer) {
        // this code asserts because the State used is not
        // the one published with this code
        assert!(state_version() == 1, 300);
    }
}
