//# run
script {
    fun main(): () {
        for (i in 10..9) {
            assert!(false, 42);
        };
    }
}
