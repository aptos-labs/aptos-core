module 0xbeef::State {
    struct State has key {
        value: u64
    }

    public fun create_state(value: u64): State {
        State {value}
    }

    public fun version(): u64 {
        0
    }
}
