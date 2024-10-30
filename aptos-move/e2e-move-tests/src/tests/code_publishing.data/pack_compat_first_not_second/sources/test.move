// This package is compatible with `pack_initial` but not with `pack_upgrade_compat`.
module publisher::test {

    struct State has key {
        value: u64
    }

    public entry fun hello(s: &signer, value: u64) {
        move_to(s, State{value})
    }

    public entry fun hello2(s: &signer) {  // different in hello2 signature
        move_to(s, State{value: 0})
    }
}
