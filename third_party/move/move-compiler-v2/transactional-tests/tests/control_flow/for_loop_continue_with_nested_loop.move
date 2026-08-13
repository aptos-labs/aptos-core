//# run
script {
    fun main() {
        // A nested `while` in the for body (with its own `continue`) plus the
        // for-loop's own `continue`. The for's `continue` must still advance the
        // iterator, and the inner `while`'s `continue` must target the `while`.
        let sum = 0;
        for (i in 0..4) {          // i = 0,1,2,3
            let k = 0;
            while (k < 2) {
                k = k + 1;
                if (k == 1) continue; // continue the inner while
                sum = sum + 10;       // runs only when k == 2 -> +10 per for iteration
            };
            if (i % 2 == 0) continue; // continue the for -> must advance i
            sum = sum + 1;            // runs only for odd i (1, 3) -> +2 total
        };
        // 4 iterations * 10 (from the while) + 2 (odd i) = 42.
        assert!(sum == 42, sum);
    }
}
