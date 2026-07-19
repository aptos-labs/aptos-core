//# run
script {
    fun main() {
        let taken = 0;      // counts how many times `continue` is taken
        let guard = 0;      // trips if the real iterator fails to advance
        for (i in 0..3) {
            guard = guard + 1;
            if (guard > 50) abort 99;   // infinite-loop detector
            let i = 100;                // shadow the iterator inside the body
            let _ = i;
            if (i == 100) {
                taken = taken + 1;
                continue                // continue while the shadow is in scope
            };
            abort 7                     // unreachable: shadow i is always 100
        };
        // Correct: loop runs exactly 3 times (i = 0,1,2); continue taken 3 times.
        // Buggy: increment hits the shadow, real i never advances -> guard trips at 51.
        assert!(taken == 3, 42);
        assert!(guard == 3, 43);
    }
}
