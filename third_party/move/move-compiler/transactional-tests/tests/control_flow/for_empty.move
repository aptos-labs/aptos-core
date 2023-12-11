//# run
script {
    fun main() {
        let x = 0;
        for (i in 10..4) {
            x = x + 1;
        };
        assert!(x == 0, 42);
    }
}
