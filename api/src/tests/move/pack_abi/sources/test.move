module abi::test {
    use std::option;

    struct State has key {
        value: u64
    }

    enum MyEnum has drop, store {
        This,
        That
    }

    #[event]
    struct MyEvent has store {
        value: u64
    }

    struct TestOption has drop, copy {
        o: option::Option<u64>
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

    #[view]
    public fun view_function(value: u64): u64 {
        value + 42
    }

    fun private_function(s: &signer, value: u64) {
        move_to(s, State { value });
    }
}
