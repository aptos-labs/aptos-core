module 0x43::C {
    use 0x41::D;
    struct T {g: D::G}
}

module 0x41::D {
    struct G {}
}

module 0x42::B {
    use 0x43::C;
    use 0x41::D::G;
    public fun foo(): C::T {
        C::T {g: G{}}
    }
    public fun bar(c: C::T) {
        let C::T { g } = c;
        let G {} = g;
    }
    public fun bar_ref(c: &C::T) {
        let C::T { g } = c;
        let G {} = g;
    }
}
