module TestAccount::Sum {
    fun sum(x: u128, y: u128): u128 {
        let sum_r = x + y;

        spec {
                // Senseless specification - mutator will change + operator to -*/ but spec won't notice it.
                assert sum_r >= 0;
        };

        sum_r
    }
}
