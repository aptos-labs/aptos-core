//# run
script {
    fun main(): () {
        let y = 0;
        for (i in 0..1 spec {invariant y > 0;}) {
            y = y + 1;
        };
        assert!(y == 1, 20);
    }
}
