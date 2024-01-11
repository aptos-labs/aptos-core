//# run
script {
    fun main() {
        let x = 0;
        for (i in 0..10) {
            if (x >= 5) break;
            x = x + 1;
            continue
        }
        assert!(move x == 5, 42);
    }
}
