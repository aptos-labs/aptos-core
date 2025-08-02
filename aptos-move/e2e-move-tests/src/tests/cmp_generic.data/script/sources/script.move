script {
    fun main(
        first: signer,
        second: signer
    ) {
            assert!(first <= second, 0);
    }
}
