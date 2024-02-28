module NamedAddr::Detector {
    use std::vector;
   // Function with correct array indexing
    public fun correct_indexing() {
        let arr = vector<u64>[1, 2, 3, 4, 5];
        let i: u64 = 2;
        vector::borrow(&arr, i);
    }

    // Function with out-of-bounds array indexing
    public fun out_of_bounds_indexing() {
        let arr = vector[1, 2, 3, 4, 5];
        vector::borrow(&arr, 10);
    }
}
