module 0x42::test {

    struct X<M> has copy, drop, store {
        value: M
    }

    fun test_vector() {
        let x = X {
            value: 2
        };
        assert!(x[0].value == 2, 0);
    }

}
