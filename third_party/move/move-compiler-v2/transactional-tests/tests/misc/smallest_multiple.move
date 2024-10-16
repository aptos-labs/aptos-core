//# publish
module 0xc0ffee::smallest_multiple {
    public fun gcd(a: u64, b: u64): u64 {
        if (b == 0) {
            return a
        } else {
            gcd(b, a % b)
        }
    }

    public fun lcm(a: u64, b: u64): u64 {
        (a * b) / gcd(a, b)
    }

    public fun smallest_multiple(limit: u64): u64 {
        let result = 1;
        let i = 1;
        while (i <= limit) {
            result = lcm(result, i);
            i = i + 1;
        };
        result
    }

    public fun test_smallest_multiple() {
        assert!(smallest_multiple(10) == 2520, 0);
        assert!(smallest_multiple(20) == 232792560, 1);
    }
}

//# run 0xc0ffee::smallest_multiple::test_smallest_multiple
