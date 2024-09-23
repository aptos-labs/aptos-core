module 0x8675309::M {
    struct S { u: u64 }
    struct R {
        f: u64
    }
    struct G0<phantom T> has drop {}
    struct G1<phantom T: key> {}
    struct G2<phantom T> {}
    struct Key has key {}

    // Leave out test cases and edit code to avoid shadowing drop errors here.
    fun t0(s: S, s2: S) {
        // (0: u8) != (1: u128);
        // 0 != false;
        // &0 != 1;
        // 1 != &0;
        s != s2;
        // s_mut != s;
    }

    fun t1(r1: R, r2: R) {
        r1 != r2;
    }

    fun t3() {
        G0<u64>{} != G0<u64>{};
        G1<Key>{} != G1<Key>{};
        G2<Key>{} != G2<Key>{};
    }
}
