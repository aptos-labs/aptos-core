module 0x8675309::M {
    const MAX_U64: u128 = 18446744073709551615;
    struct R<T: key>  { r: T }
    struct X<T> has key, drop {
        r: T
    }

    fun t0() {
        let y = R { r: X { r: 0} };
        let R { r: _r } = y;
    }

    fun t0_u128() {
        let y = R { r: X { r: MAX_U64 + 1} };
        let R { r: _r } = y;
    }
}
