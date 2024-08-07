module TestAccount::Sum {
    fun sum(x: u128, y: u128): u128 {
        let sum_r = x + y;
        spec {
                assert sum_r == x+y;
        };

        sum_r
    }
}
