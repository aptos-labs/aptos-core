module 0xc0ffee::m {
    public enum E<T1, T2> has drop {
        A { x: T1 },
        B { x: T2 },
        C { x: T1 },
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m::E;

    fun select(e: &E<u64, u64>): u64 {
        e.x
    }

    fun variant_qualified(e: &E<u64, u64>): u64 {
        if (e is E::B) e.B.x else e.A.x
    }

    fun matched(e: &E<u64, u64>): u64 {
        match (e) {
            E::A { x } => *x,
            E::B { x } => *x,
            E::C { x } => *x,
        }
    }
}
