//# run
script {
    fun main() {
        let x = 0;
        for (i in true..false) {
            x = x + 1;
        };
        assert!(x == 0, 42);
    }
}
