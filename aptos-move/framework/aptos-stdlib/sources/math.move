/// Standard math utilities missing in the Move Language.
module aptos_std::math {

    /// Return the largest of two numbers.
    public fun max(a: u64, b: u64): u64 {
        if (a >= b) a else b
    }

    /// Return the smallest of two numbers.
    public fun min(a: u64, b: u64): u64 {
        if (a < b) a else b
    }

    /// Return the average of two.
    public fun average(a: u64, b: u64): u64 {
        (a & b) + (a ^ b) / 2
    }

    /// Return the value of n raised to power e
    public fun pow(n: u64, e: u64): u64 {
        if (e == 0) {
            1
        } else if (e == 1) {
            n
        } else {
            let p = pow(n, e / 2);
            p = p * p;
            if (e % 2 == 1) {
                p = p * n;
                p
            } else {
                p
            }
        }
    }

    #[test]
    public entry fun test_max() {
        let result = max(3, 6);
        assert!(result == 6, 0);

        let result = max(15, 12);
        assert!(result == 15, 1);
    }

    #[test]
    public entry fun test_min() {
        let result = min(3, 6);
        assert!(result == 3, 0);

        let result = min(15, 12);
        assert!(result == 12, 1);
    }

    #[test]
    public entry fun test_average() {
        let result = average(3, 6);
        assert!(result == 4, 0);

        let result = average(15, 12);
        assert!(result == 13, 0);
    }

    #[test]
    public entry fun test_pow() {
        let result = pow(10, 18);
        assert!(result == 1000000000000000000, 0);

        let result = pow(10, 1);
        assert!(result == 10, 0);

        let result = pow(10, 0);
        assert!(result == 1, 0);
    }
}
