module 0x42::M {
    enum M has drop {
        M
    }
}

module 0x42::M1 {
    use 0x42::M::{Self, M};

    fun test(_m: M::M): u64 {
        3
    }
}
