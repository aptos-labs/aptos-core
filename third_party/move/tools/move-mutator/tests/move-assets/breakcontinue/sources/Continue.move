module TestAccount::Continue {
    fun sum_intermediate(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            if (i % 10 == 0) continue;
            sum = sum + i;
        };

        sum
    }
}
