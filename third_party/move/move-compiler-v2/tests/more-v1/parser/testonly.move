module 0x1::A {
    #[test]
    fun a() { }

    #[testonly]
    public fun a_call() {
        abort 0
    }

    #[test_only]
    public fun b_call() {
        abort 0
    }

    #[view]
    public fun c_call() {
        abort 0
    }
}
