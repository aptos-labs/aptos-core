module 0x42::test {
    fun unused_assignment_in_loop() {
        let x = 1;
        for (i in 0..10) {
            x = x + 1;
        };
    }
}
