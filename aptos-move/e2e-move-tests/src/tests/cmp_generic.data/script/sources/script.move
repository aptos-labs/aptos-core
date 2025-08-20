script {
    fun main(
        first: signer,
        second: signer
    ) {
            let a = &first;
            let b = &second;
            assert!(a <= b && first <= second, 0);
    }
}
