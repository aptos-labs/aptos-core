//# publish
module 0xc0ffee::sum_multiples {
    public fun sum_multiples_3_or_5(limit: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < limit) {
            if (i % 3 == 0 || i % 5 == 0) {
                sum = sum + i;
            };
            i = i + 1;
        };
        sum
    }

    public fun test_sum_multiples() {
        assert!(sum_multiples_3_or_5(10) == 23, 0);
        assert!(sum_multiples_3_or_5(1000) == 233168, 1);
    }
}

//# run 0xc0ffee::sum_multiples::test_sum_multiples
