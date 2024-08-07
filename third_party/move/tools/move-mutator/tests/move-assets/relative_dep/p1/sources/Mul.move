module TestAccount::Mul {
    public fun mul(x: u128, y: u128): u128 {
        let mult = x * y;
        spec {
            assert mult == x*y;
        };

        mult
    }
}
