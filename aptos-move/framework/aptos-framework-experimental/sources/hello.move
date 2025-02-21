module aptos_framework_experimental::hello {
    use std::hash;

    public fun spend_some_gas_for_nothing() {
        let _ = hash::sha3_256(b"some awesome hash input");
    }

    #[test]
    fun some_test() {
        spend_some_gas_for_nothing();
    }
}
