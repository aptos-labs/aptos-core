module abi::test {

    struct State has key {
        value: u64
    }

    public fun public_function(s: &signer, state: State) {
        move_to(s, state)
    }

    public entry fun public_entry_function(s1: &signer, s2: &signer, value: u64) {
        move_to(s1, State { value });
        move_to(s2, State { value });
    }

    entry fun private_entry_function(s: &signer, value: u64) {
        move_to(s, State { value });
    }

    fun private_function(s: &signer, value: u64) {
        move_to(s, State { value });
    }
}
