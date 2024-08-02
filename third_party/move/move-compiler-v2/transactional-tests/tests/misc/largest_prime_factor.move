//# publish
module 0xc0ffee::largest_prime_factor {
    public fun is_prime(n: u64): bool {
        if (n < 2) {
            return false
        };

        let i = 2;
        while (i <= n / 2) {
            if (n % i == 0) {
                return false
            };
            i = i + 1;
        };

        true
    }

    public fun largest_prime_factor(n: u64): u64 {
        let largest_factor = 1;
        let i = 2;
        while (i <= n / 2) {
            if (n % i == 0 && is_prime(i)) {
                largest_factor = i;
            };
            i = i + 1;
        };

        if (is_prime(n)) {
            largest_factor = n;
        };

        largest_factor
    }

    public fun test_largest_prime_factor() {
        assert!(largest_prime_factor(13195) == 29, 0);
    }
}

//# run 0xc0ffee::largest_prime_factor::test_largest_prime_factor
