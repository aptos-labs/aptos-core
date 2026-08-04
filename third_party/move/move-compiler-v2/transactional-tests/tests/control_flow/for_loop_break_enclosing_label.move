//# run
script {
    fun main() {
        // `break 'outer` from inside a `for` targets the enclosing labeled loop
        // (the for-loop sits between the break and its target, so lowering must
        // shift the continuation out by one for the extra inner loop level).
        let count = 0;
        'outer: while (true) {
            for (i in 0..10) {
                count = count + 1;
                if (count >= 3) break 'outer;
            };
            count = count + 1000; // unreachable: the for always breaks out first
        };
        assert!(count == 3, 42);
    }
}
