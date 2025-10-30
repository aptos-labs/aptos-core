module supra_std::vector_utils {

    use std::error;
    use std::vector;


    /// The index into the vector is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 0;
    /// Input vectors length does not match.
    // std::vector has error constant which value is 0x20002 for similar cases,
    // reporting this error via error::out_or_range will result to the same value.
    const EVECTORS_LENGTH_MISMATCH: u64 = 2;

    /// Replace the `i`th element of the vector `v` with the input element.
    /// This is O(1), but does preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun replace<Element>(v: &mut vector<Element>, i: u64, element: Element): Element {
        assert!(i < vector::length(v), error::out_of_range(EINDEX_OUT_OF_BOUNDS));
        vector::push_back(v, element);
        vector::swap_remove(v, i)
    }

    /// Sorts values in ascending order.
    public fun sort_vector_u64(values: vector<u64>) : vector<u64> {
        native_sort_vector_u64(values)
    }

    /// Sorts values based on the input keys in ascending order.
    /// The keys and values should match in length, otherwise function will abort.
    public fun sort_vector_u64_by_keys(keys: vector<u64>, values: vector<u64>) : vector<u64> {
        assert!(vector::length(&keys) == vector::length(&values), error::out_of_range(EVECTORS_LENGTH_MISMATCH));
        native_sort_vector_u64_by_key(keys, values)
    }

    /// Sorts values in ascending order.
    native fun native_sort_vector_u64(values: vector<u64>): vector<u64>;

    /// Sorts values based on the input keys in ascending order.
    native fun native_sort_vector_u64_by_key(keys: vector<u64>, values: vector<u64>): vector<u64>;


    #[test_only]
    fun create_vector(size: u64): vector<u64> {
        let i = 0;
        let result = vector<u64>[];
        while (i < size) {
            if (i % 5 > 3) {
                vector::push_back(&mut result, i)
            } else {
                vector::insert(&mut result, 0, i)
            };
            i = i + 1
        };
        result
    }

    #[test]
    fun check_vector_u64_sort() {
        let values = create_vector(500);
        let sorted_values = sort_vector_u64(values);
        assert!(vector::length(&sorted_values) == 500, 1);
    }

    #[test]
    fun check_vector_u64_sort_by_key() {
        let values = create_vector(5);
        let keys = create_vector(5);
        let sorted_values = sort_vector_u64_by_keys(keys, values);
        assert!(vector::length(&sorted_values) == 5, 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun check_vector_u64_sort_by_key_failure() {
        let values = create_vector(5);
        let keys = create_vector(4);
        sort_vector_u64_by_keys(keys, values);
    }

    #[test]
    fun check_vector_replace() {
        let values = create_vector(5);
        replace(&mut values, 2, 100000);
        assert!(*vector::borrow(&values, 2) == 100000, 1);
        replace(&mut values, 4, 100000);
        assert!(*vector::borrow(&values, 4) == 100000, 2);
    }

    #[test]
    #[expected_failure(abort_code = 0x20000, location = Self)]
    fun check_replace_with_invalid_index() {
        let values = create_vector(5);
        replace(&mut values, 5, 100000);
    }
}
