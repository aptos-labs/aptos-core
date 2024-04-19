//# run
script {
    fun main(): () {
        let y = 0;
        for (i in 0..10) {
            y = y + 1;
            i = i + 1;
        };
        assert!(y == 5, 42);
    }
}
