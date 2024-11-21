module 0x8675309::M {
    // struct Coin {}
    struct R<T: key>  { r: T }
    struct X<T> has key, drop {
        r: T
    }
    // struct S<T: drop> has drop { c: T }

    fun t0() {
        let y = R { r: X { r: 0} };
        let R { r: _r } = y;
        // S { c: Coin {} };
    }
}
