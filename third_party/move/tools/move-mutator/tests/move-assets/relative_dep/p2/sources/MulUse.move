module CafeAccount::MulUse {
    use TestAccount::Mul::mul;

    fun mul_usage(x: u128, y: u128): u128 {
        let z = x + 1;
        let w = y + 2;
        mul(z, w)
    }
}
