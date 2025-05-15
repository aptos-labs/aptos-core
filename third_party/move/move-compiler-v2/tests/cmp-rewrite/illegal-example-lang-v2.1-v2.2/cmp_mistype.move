module 0xcffa::m {
    public fun eq<T1: copy + drop, T2: copy + drop>(x: T1, y: T2): bool {
        x == y
    }
}

module 0xcffb::m {
    public fun eq<T1: copy + drop, T2: copy + drop>(x: T1, y: T2): bool {
        &x == &y
    }
}

module 0xcffc::m {
    public fun eq(x: u8, y: u16): bool {
        x == y
    }
}

module 0xcffd::m {
    public fun eq(x: u256, y: address): bool {
        x == y
    }
}

module 0xcffe::m {
    public fun eq(x: vector<u8>, y: &vector<u8>): bool {
        x == y
    }
}

module 0xcfff::m {
    struct Foo has copy, drop { x: u64, y: bool }
    public fun eq(x: Foo, y: &Foo): bool {
        x == y
    }
}

module 0xdffa::m {
    public fun neq<T1: copy + drop, T2: copy + drop>(x: T1, y: T2): bool {
        x != y
    }
}

module 0xdffb::m {
    public fun neq<T1: copy + drop, T2: copy + drop>(x: T1, y: T2): bool {
        &x != &y
    }
}

module 0xdffc::m {
    public fun neq(x: u8, y: u16): bool {
        x != y
    }
}

module 0xdffd::m {
    public fun neq(x: u256, y: address): bool {
        x != y
    }
}

module 0xdffe::m {
    public fun neq(x: vector<u8>, y: &vector<u8>): bool {
        x != y
    }
}

module 0xdfff::m {
    struct Foo has copy, drop  { x: u64, y: bool }
    public fun neq(x: Foo, y: &Foo): bool {
        x != y
    }
}
