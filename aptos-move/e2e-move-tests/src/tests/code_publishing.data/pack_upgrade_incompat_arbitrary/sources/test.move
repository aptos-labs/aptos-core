module publisher::test {

    struct State has key {
        value: u64
    }

    public entry fun hello(s: &signer, value: u64, _incompat_additional: u64) {
        move_to(s, State{value})
    }
}
