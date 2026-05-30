module 0x42::generic_call {
    fun identity<T>(x: T): T {
        x
    }

    fun call_u64(): u64 {
        identity<u64>(7)
    }

    fun call_bool(): bool {
        identity<bool>(true)
    }
}
