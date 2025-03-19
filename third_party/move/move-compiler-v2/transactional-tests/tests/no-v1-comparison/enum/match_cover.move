//# publish
module 0xc0ffee::m {
    enum A has drop {
        V1 { a: Q, b: R},
    }

    enum Q has drop {
        Q1,
        Q2,
    }

    enum R has drop {
        R1,
        R2,
    }

    public fun test1(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 { a: _, b: R::R1 } => {},
            A::V1 {a: Q::Q2, b: R::R2} => {},
        }
    }

    public fun test2(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 { a: _, b: R::R1 } => {},
            A::V1 {..} => {},
        }
    }

    public fun test3(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 { a: _, b: R::R1 } => {},
            _ => {},
        }
    }

    public fun test4(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 {..} => {},
        }
    }

    public fun test5(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 { a: Q::Q2, b: _ } => {},
        }
    }

    public fun test6(a: A) {
        match (a) {
            A::V1 { a: Q::Q1, b: _ } => {},
            A::V1 { a: Q::Q2, b: _ } if true => {},
            _ => {},
        }
    }
}


//# publish
module 0xc0ffee::n {
    enum A has drop {
        V1 { a: P, b: Q, c: R},
    }

    enum P has drop {
        P1,
        P2,
    }

    enum Q has drop {
        Q1,
        Q2,
    }

    enum R has drop {
        R1,
        R2,
    }

    public fun test(a: A) {
        match (a) {
            A::V1 { a: P::P1, b: _, c: _ } => {},
            A::V1 { a: _, b: Q::Q1, c: _ } => {},
            A::V1 { a: _, b: _, c: R::R1 } => {},
            A::V1 { a: P::P2, b: Q::Q2, c: R::R2 } => {},
        }
    }
}


//# publish
module 0xc0ffee::o {
    enum E has drop {
        V1 { a: F, b: G },
        V2 { a: F, b: G, c: H }
    }

    enum F has drop {
        F1,
        F2 { a: G }
    }

    enum G has drop {
        G1 { a: H, b: H },
        G2 { a: H }
    }

    enum H has drop {
        H1 { a: u64},
        H2 { b: u64 }
    }

    public fun test1(e: E) {
        match (e) {
            E::V1 {b: _, ..} => {},
            E::V2 {..} => {}
        }
    }

    public fun test2(e: E) {
        match (e) {
            E::V1 {b: G::G1{ a: H::H1 { a: _}, b: _}, ..} => {},
            E::V1 {b: G::G1{ a: _, b: H::H1 { .. }}, ..} => {},
            E::V1 {b: _, a: F::F1} => {},
            E::V1 {b: _, a: F::F2 {..} } => {},
            E::V2 {..} => {}
        }
    }

    public fun test3(e: E) {
        match (e) {
            E::V1 {b: G::G1{ .. }, a: _} => {},
            E::V1 {b: _, a: F::F2 {..} } => {},
            E::V1 {b: G::G2{ a: H::H2 {b: _}}, a: F::F1} => {},
            E::V1 {b: _, a: F::F1} => {},
            E::V2 {..} => {}
        }
    }
}

//# publish
module 0xc0ffee::o_fail {
    enum E has drop {
        V1 { a: F, b: G },
        V2 { a: F, b: G, c: H }
    }

    enum F has drop {
        F1,
        F2 { a: G }
    }

    enum G has drop {
        G1 { a: H, b: H },
        G2 { a: H }
    }

    enum H has drop {
        H1 { a: u64},
        H2 { b: u64 }
    }

    public fun test1(e: E) {
        match (e) {
            E::V1 {b: _, ..} => {},
            E::V1 {..} => {}
            E::V2 {..} => {}
        }
    }

    public fun test2(e: E) {
        match (e) {
            E::V2 {..} => {}
            E::V1 {b: G::G1{ .. }, a: _} => {},
            E::V1 {b: _, a: F::F2 {..} } => {},
            E::V1 {b: _, a: F::F1} => {},
            E::V1 {b: G::G2{ a: H::H2 {b: _}}, a: F::F1} => {},
        }
    }

    public fun test3(e: E) {
        match (e) {
            E::V1 {b: G::G1{ .. }, a: F::F1} => {},
            E::V1 {b: G::G2{ .. }, a: F::F2 {..}} => {},
            E::V1 {a: F::F2 {..}, ..} => {},
            E::V1 {b: _, a: F::F1} => {},
            E::V1 {..} => {}
            E::V2 {..} => {}
        }
    }
}
