/// An infinite loop that stops when its gas budget is exhausted.
module fixtures::spin {
    #[test]
    fun forever() {
        loop {}
    }
}
