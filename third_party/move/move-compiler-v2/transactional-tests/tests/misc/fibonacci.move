//# publish
module 0x42::fibonacci {
    public fun fib(n: u64): u64 {
        if (n == 0) {
            return 0
        } else if (n == 1) {
            return 1
        } else {
            return fib(n - 1) + fib(n - 2)
        }
    }

    public fun test_fib() {
        assert!(fib(0) == 0, 0);
        assert!(fib(1) == 1, 1);
        assert!(fib(2) == 1, 2);
        assert!(fib(3) == 2, 3);
        assert!(fib(4) == 3, 4);
        assert!(fib(5) == 5, 5);
        assert!(fib(6) == 8, 6);
        assert!(fib(7) == 13, 7);
        assert!(fib(8) == 21, 8);
        assert!(fib(9) == 34, 9);
        assert!(fib(10) == 55, 10);
    }
}

//# run 0x42::fibonacci::test_fib
