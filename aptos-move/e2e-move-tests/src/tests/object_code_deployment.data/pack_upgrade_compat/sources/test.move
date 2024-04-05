module object::test {

    struct State has key {
        value: u64
    }

    public entry fun hello(s: &signer, value: u64) {
        move_to(s, State { value })
    }

    public entry fun hello2(s: &signer, value: u64) {
        move_to(s, State { value })
    }
}
