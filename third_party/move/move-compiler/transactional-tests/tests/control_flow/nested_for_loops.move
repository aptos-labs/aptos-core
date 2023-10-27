//# run
script {
    fun main(): () {
        let y = 0;
        for (i in 0..10) {
            y = y + 1;
            for (j in i..10) {
              y = y + 1;
            };
        };
        assert!(y == 65, 42);
    }
}
