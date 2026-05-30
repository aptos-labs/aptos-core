module 0x42::generic_struct_field {
    struct Box<T> has copy, drop {
        value: T,
    }

    fun make_u64(v: u64): Box<u64> {
        Box<u64> { value: v }
    }

    fun get_u64(b: &Box<u64>): u64 {
        b.value
    }
}
