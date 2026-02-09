module 0x42::m {
    use std::vector;

    // Used as type parameter in function call
    struct UsedAsTypeParam has drop {
        x: u64
    }

    // Used as type parameter in vector operations
    struct UsedInVectorOps has drop {
        y: u64
    }

    // Unused struct for comparison
    struct ReallyUnused {
        z: u64
    }

    // Helper function with type parameter
    fun helper<T: drop>(_x: T) {
    }

    public fun test_type_param() {
        // Call helper with struct as type parameter
        helper<UsedAsTypeParam>(UsedAsTypeParam { x: 1 });
    }

    public fun test_vector_ops() {
        // Create empty vector with struct type parameter
        let _v = vector::empty<UsedInVectorOps>();
        vector::push_back(&mut _v, UsedInVectorOps { y: 2 });
    }
}
