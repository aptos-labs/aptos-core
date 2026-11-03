module 0x42::m {
    struct S<phantom T> has copy, drop {}

    enum Uniform has copy, drop {
        A { x: u64 },
        B { x: u64 },
    }

    enum UniformGeneric<T> has copy, drop {
        A { x: T },
        B { x: T },
    }

    enum E<T1, T2> has copy, drop {
        A { x: T1 },
        B { x: T2 },
    }

    enum Partial<T1, T2> has copy, drop {
        A { x: T1, y: u64 },
        B { y: u64 },
        C { x: T1 },
        D { x: T2 },
    }

    enum Phantom<phantom T1, phantom T2> has copy, drop {
        A { x: S<T1> },
        B { x: S<T2> },
    }

    enum Concrete has copy, drop {
        A { x: u64 },
        B { x: u8 },
    }

    // Same shape as `std::result::Result`.
    enum Result<T, E> has copy, drop {
        Ok(T),
        Err(E),
    }

    fun uniform(e: &Uniform): u64 {
        e.x
    }

    fun uniform_generic<T: copy>(e: &UniformGeneric<T>): T {
        e.x
    }

    fun coinciding_concrete(e: &E<u8, u8>): u8 {
        e.x
    }

    fun coinciding_generic<T: copy + drop>(e: &E<T, T>): T {
        e.x
    }

    // Caller's type parameter names clash with the enum's.
    fun swapped<T1: copy + drop, T2: copy + drop>(e: &E<T2, T1>): T2 {
        e.x
    }

    fun differing(e: &E<u8, u64>): u8 {
        e.x
    }

    fun make<T1: drop, T2: drop>(a: T1): E<T1, T2> {
        E::A { x: a }
    }

    fun inferred_before(): u8 {
        let e: E<u8, u8> = make(1u8);
        e.x
    }

    fun inferred_after(): u8 {
        let e = make(1u8);
        let y = e.x;
        let _check: E<u8, u8> = e;
        y
    }

    fun partial_uniform(e: &Partial<u8, u8>): u64 {
        e.y
    }

    fun partial_coinciding(e: &Partial<u8, u8>): u8 {
        e.x
    }

    fun phantom(e: &Phantom<u8, u8>): S<u8> {
        e.x
    }

    fun concrete(e: &Concrete): u64 {
        e.x
    }

    fun result_select(r: &Result<u64, u64>): u64 {
        r.0
    }

    fun variant_qualified(e: &E<u8, u8>): u8 {
        if (e is A) e.A.x else e.B.x
    }

    spec variant_qualified(e: &E<u8, u8>): u8 {
        ensures (e is A) ==> result == e.A.x;
        ensures (e is B) ==> result == e.B.x;
        ensures result == e.x;
    }

    fun matched(e: &E<u8, u8>): u8 {
        match (e) {
            E::A { x } => *x,
            E::B { x } => *x,
        }
    }
}
