module 0x8675309::M {
    struct S { u: u64 }
    struct R has key {
        f: u64
    }
    struct G0<T> has drop { f: T }
    struct G1<T: key> { f: T }
    struct G2<phantom T> has drop {}

    fun t1(r: R) {
        r == r;
    }

    fun t3<T: copy + key>(t: T) {
        G1{ f: t } == G1{ f: t };
    }
}
