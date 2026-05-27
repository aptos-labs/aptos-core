module 0x66::vec_pushback_safe_point {
    use std::vector;

    fun fresh(): vector<u8> {
        vector::empty<u8>()
    }

    fun take(_v: vector<u8>) {}

    public fun caller() {
        let saved = fresh();
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 42);
        take(saved);
    }
}
