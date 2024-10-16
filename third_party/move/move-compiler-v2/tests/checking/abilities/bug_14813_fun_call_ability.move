module 0x815::red_black_map {

    struct Map<phantom V> has drop {}

    public fun contains<V>(self: &Map<V>, key: u256): bool {
        self.search(key)
    }

    fun search<V: drop>(self: &Map<V>, key: u256): bool {
        true
    }
}
