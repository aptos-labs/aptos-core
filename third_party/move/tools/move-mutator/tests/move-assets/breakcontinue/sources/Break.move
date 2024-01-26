module TestAccount::Break {
    fun smallest_factor(n: u64): u64 {
        // assuming the input is not 0 or 1
        let i = 2;
        while (i <= n) {
            if (n % i == 0) break;
            i = i + 1
        };

        i
    }
}
