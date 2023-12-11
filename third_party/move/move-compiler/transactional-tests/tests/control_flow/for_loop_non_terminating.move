//# run --gas-budget 700
script {
    fun main(): () {
        let y = 0;
        // out of gas
        for (i in 0..10) {
            i = 0;
        };
        assert!(false, 42);
    }
}
