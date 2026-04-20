// Tests that lambdas cannot be passed where a type requiring `store` ability is expected.
module 0xa::test {

    // Requires store: lambda cannot satisfy store
    fun require_store<T: store>(_x: T) { }

    fun bad_store() {
        require_store(|| 1u64);
    }
}
