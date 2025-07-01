//# publish
module 0xc0ffee::m {
    public enum A has drop {
        V1 { a: Q, b: R},
    }

    public enum Q has drop {
        Q1,
        Q2,
    }

    public enum R has drop {
        R1,
        R2,
    }

}

//# publish
module 0xc0ffee::test_m {
    use 0xc0ffee::m::A;
    use 0xc0ffee::m::Q;
    use 0xc0ffee::m::R;

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
    public enum A has drop {
        V1 { a: P, b: Q, c: R},
    }

    public enum P has drop {
        P1,
        P2,
    }

    public enum Q has drop {
        Q1,
        Q2,
    }

    public enum R has drop {
        R1,
        R2,
    }

}

//# publish
module 0xc0ffee::test_n {
    use 0xc0ffee::n::A;
    use 0xc0ffee::n::P;
    use 0xc0ffee::n::Q;
    use 0xc0ffee::n::R;

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
    public enum E has drop {
        V1 { a: F, b: G },
        V2 { a: F, b: G, c: H }
    }

    public enum F has drop {
        F1,
        F2 { a: G }
    }

    public enum G has drop {
        G1 { a: H, b: H },
        G2 { a: H }
    }

    public enum H has drop {
        H1 { a: u64},
        H2 { b: u64 }
    }

}

//# publish
module 0xc0ffee::test_o {
    use 0xc0ffee::o::E;
    use 0xc0ffee::o::F;
    use 0xc0ffee::o::G;
    use 0xc0ffee::o::H;

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
    public enum E has drop {
        V1 { a: F, b: G },
        V2 { a: F, b: G, c: H }
    }

    public enum F has drop {
        F1,
        F2 { a: G }
    }

    public enum G has drop {
        G1 { a: H, b: H },
        G2 { a: H }
    }

    public enum H has drop {
        H1 { a: u64},
        H2 { b: u64 }
    }

}

//# publish
module 0xc0ffee::test_o_fail {
    use 0xc0ffee::o_fail::E;
    use 0xc0ffee::o_fail::F;
    use 0xc0ffee::o_fail::G;
    use 0xc0ffee::o_fail::H;

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
