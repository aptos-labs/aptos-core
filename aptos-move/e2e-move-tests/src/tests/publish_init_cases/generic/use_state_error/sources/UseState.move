module 0xbeef::UseState {
    use 0xbeef::State;

    public fun version(): u64 {
        0
    }

    public fun state_version(): u64 {
        State::version()
    }

    fun init_module(_s: &signer) {
        // this leads to a verifier error when the init_module is
        // "loaded" (`load_function`) because State used is v0,
        // which does not contain `something_new`.
        // Make sure the publishing fails, so we alert the user
        assert!(State::something_new() == 22, 200);
    }
}
