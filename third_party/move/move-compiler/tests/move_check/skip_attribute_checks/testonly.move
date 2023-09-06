module 0x1::A {
    #[test]
    fun a() { }

    #[testonly]
    public fun a_call() {
        abort 0
    }
}
