module 0xbeef::State {
    struct State has key {
        value: u64
    }

    public fun create_state(value: u64): State {
        State {value}
    }

    public fun something_new(): u64 {
        22
    }

    public fun version(): u64 {
        1
    }
}
