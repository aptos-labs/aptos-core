module 0x815::demo {
    struct S<T: store> has store {
        field: T,
    }

    struct E<T: store> has store, drop {
        entry: S<T>,
    }
}
