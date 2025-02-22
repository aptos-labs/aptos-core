/// A placeholder module.
module framework_experimental::veiled_coin {
    use std::hash;

    /// A placeholder function.
    public fun some_func() {
        let _ = hash::sha3_256(b"some input");
    }

    #[test]
    fun some_test() {
        some_func();
    }
}
