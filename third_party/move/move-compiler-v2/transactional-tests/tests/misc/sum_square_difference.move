//# publish
module 0xc0ffee::sum_square_difference {
    public fun sum_of_squares(n: u64): u64 {
        let sum = 0;
        let i = 1;
        while (i <= n) {
            sum = sum + (i * i);
            i = i + 1;
        };
        sum
    }

    public fun square_of_sum(n: u64): u64 {
        let sum = (n * (n + 1)) / 2;
        sum * sum
    }

    public fun difference(n: u64): u64 {
        square_of_sum(n) - sum_of_squares(n)
    }

    public fun test_difference() {
        assert!(difference(10) == 2640, 0);
        assert!(difference(100) == 25164150, 1);
    }
}

//# run 0xc0ffee::sum_square_difference::test_difference
