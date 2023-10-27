//# run
script {
    fun main(): () {
        let y = 0;
        for (i in 0..5) {
            y = y + 1;
            while (y < 5) {
                y = y + 10;
            };
        };
        assert!(y == 15, 42);

        let z = 0;
        while (z < 6) {
            for (j in 0..5) {
                z = z + 1;
            };
        };
        assert!(z == 10, 43);
    }
}
