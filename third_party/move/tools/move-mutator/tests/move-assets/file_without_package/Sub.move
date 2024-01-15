module Test::Sub {
    fun sub(x: u128, y: u128): u128 {
        let sub_r = x - y;
        spec {
                assert sub_r == x-y;
        };

        sub_r
    }
}
