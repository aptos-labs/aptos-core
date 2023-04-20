module 0x1::test_bv {
    const MAX_FRACTIONAL_PART: u128 = 0xFFFFFFFFFFFFFFFF;

    public fun test(x: u128, y: u128): u128 {
        let xf = x & MAX_FRACTIONAL_PART;
        let yf = y & MAX_FRACTIONAL_PART;
        ((xf * yf) >> 64)
    }
}
