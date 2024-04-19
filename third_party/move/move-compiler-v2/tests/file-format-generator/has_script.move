script {
    const FOO: u64 = 1;

    fun whatever() {
        assert!(FOO == 1, 1);
    }
}
