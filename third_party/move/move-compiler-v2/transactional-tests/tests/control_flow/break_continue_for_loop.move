//# run
script {
    fun main() {
        let x = 0;
        for (i in 0..10) { // 0,  1,  2,  3,  4, 5, 6, 7, 8, 9, 10
            x = x + 1;     // 1,  4, 13, 16,
            if (x >= 15) break;
	    x = x + 2;     // 3,  6, 15,  -
            if (i % 2 == 0) continue;
	    x = x * 2;     // -, 12,  -,  -
        };
        assert!(x == 16, x);
    }
}
