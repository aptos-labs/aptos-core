module aptos_std::big_vector {
    use std::error;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_std::type_info;

    /// Vector index is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 1;
    /// Vector is full
    const EOUT_OF_CAPACITY: u64 = 2;
    /// Cannot destroy a non-empty vector
    const EVECTOR_NOT_EMPTY: u64 = 3;
    /// The given fixed-size type can not fit in an optimized vector
    const E_TOO_BIG_TO_OPTIMIZE: u64 = 4;

    /// Optimal sector size, in bytes. Each bucket is stored as a
    /// vector, which is serialized and stored on disk as a binary large
    /// object (BLOB). Per the International Disk Drive Equipment and
    /// Materials Association (IDEMA) Advanced Format (AF) standard
    /// instituted circa 2010, newer machines are likely to implement a
    /// 4096-byte (4K) disk sector size. Hence buckets size is matched
    /// to this figure for optimized read/write performance.
    const OPTIMAL_BLOB_SIZE: u64 = 4096;
    /// Per `aptos_std::type_info`, a vector containing n fixed-width
    /// elements, each of size s, has a size of n * s + 1 bytes, when
    /// n is less than 128.
    const VECTOR_BASE_SIZE_SMALL: u64 = 1;
    /// Per `aptos_std::type_info`, a vector containing n fixed-width
    /// elements, each of size s, has a size of n * s + 2 bytes, when
    /// 128 <= n < 16384
    const VECTOR_BASE_SIZE_LARGE: u64 = 2;
    /// Per `aptos_std::type_info`, the base size of a vector
    /// increases to 2 bytes when the 127th element is added
    const VECTOR_BASE_SIZE_LENGTH_CUTOFF: u64 = 127;

    /// Index of the value in the buckets.
    struct BigVectorIndex has copy, drop, store {
        bucket_index: u64,
        vec_index: u64,
    }

    /// A Scalable vector implementation based on tables, elements are grouped into buckets with `bucket_size`.
    struct BigVector<T> has store {
        buckets: TableWithLength<u64, vector<T>>,
        end_index: BigVectorIndex,
        num_buckets: u64,
        bucket_size: u64
    }

    /// Regular Vector API

    /// Create an empty vector.
    public fun new<T: store>(bucket_size: u64): BigVector<T> {
        assert!(bucket_size > 0, 0);
        BigVector {
            buckets: table_with_length::new(),
            end_index: BigVectorIndex {
                bucket_index: 0,
                vec_index: 0,
            },
            num_buckets: 0,
            bucket_size,
        }
    }

    /// Create an empty vector with `num_buckets` reserved.
    public fun new_with_capacity<T: store>(bucket_size: u64, num_buckets: u64): BigVector<T> {
        let v = new(bucket_size);
        reserve(&mut v, num_buckets);
        v
    }

    /// Create a new optimized vector, provided a reference to a
    /// quasi-null instance of fixed-size element type `T`. See inner
    /// function `get_optimized_bucket_size()`.
    public fun new_optimized<T: store>(
        fixed_size_type_null_ref: &T
    ): BigVector<T> {
        // Return optimized vector
        new(get_optimal_bucket_size(fixed_size_type_null_ref))
    }

    /// Create an empty optimized vector with `num_buckets`, provided a
    /// reference to a quasi-null instance of fixed-size element type
    /// `T`. See inner function `get_optimized_bucket_size()`.
    public fun new_with_capacity_optimized<T: store>(
        fixed_size_type_null_ref: &T,
        num_buckets: u64
    ): BigVector<T> {
        // Return new optimized vector with specified number of buckets
        new_with_capacity(
            get_optimal_bucket_size(fixed_size_type_null_ref), num_buckets)
    }

    /// Return a singleton optimized vector containing `element`.
    public fun singleton_optimized<T: store>(
        element: T
    ): BigVector<T> {
        // Create new empty optimized vector
        let vector_optimized = new_optimized(&element);
        // Insert element
        push_back(&mut vector_optimized, element);
        vector_optimized // Return optimized singleton
    }

    /// Return a singleton optimized vector containing `element`, with
    /// `num_buckets` reserved.
    public fun singleton_with_capacity_optimized<T: store>(
        element: T,
        num_buckets: u64
    ): BigVector<T> {
        // Create new empty optimized vector with reserved buckets
        let vector_optimized =
            new_with_capacity_optimized(&element, num_buckets);
        // Insert element
        push_back(&mut vector_optimized, element);
        vector_optimized // Return optimized singleton
    }

    /// Destroy the vector `v`.
    /// Aborts if `v` is not empty.
    public fun destroy_empty<T>(v: BigVector<T>) {
        assert!(is_empty(&v), error::invalid_argument(EVECTOR_NOT_EMPTY));
        shrink_to_fit(&mut v);
        let BigVector { buckets, end_index: _, num_buckets: _, bucket_size: _ } = v;
        table_with_length::destroy_empty(buckets);
    }

    /// Add element `val` to the end of the vector `v`. It grows the buckets when the current buckets are full.
    /// This operation will cost more gas when it adds new bucket.
    public fun push_back<T>(v: &mut BigVector<T>, val: T) {
        if (v.end_index.bucket_index == v.num_buckets) {
            table_with_length::add(&mut v.buckets, v.num_buckets, vector::empty());
            v.num_buckets = v.num_buckets + 1;
        };
        vector::push_back(table_with_length::borrow_mut(&mut v.buckets, v.end_index.bucket_index), val);
        increment_index(&mut v.end_index, v.bucket_size);
    }

    /// Add element `val` to the end of the vector `v`.
    /// Aborts if all buckets are full.
    /// It can split the gas responsibility between user of the vector and owner of the vector.
    /// Call `reserve` to explicit add more buckets.
    public fun push_back_no_grow<T>(v: &mut BigVector<T>, val: T) {
        assert!(v.end_index.bucket_index < v.num_buckets, error::invalid_argument(EOUT_OF_CAPACITY));
        vector::push_back(table_with_length::borrow_mut(&mut v.buckets, v.end_index.bucket_index), val);
        increment_index(&mut v.end_index, v.bucket_size);
    }

    /// Pop an element from the end of vector `v`. It doesn't shrink the buckets even if they're empty.
    /// Call `shrink_to_fit` explicity to deallocate empty buckets.
    /// Aborts if `v` is empty.
    public fun pop_back<T>(v: &mut BigVector<T>): T {
        assert!(!is_empty(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        decrement_index(&mut v.end_index, v.bucket_size);
        let val = vector::pop_back(table_with_length::borrow_mut(&mut v.buckets, v.end_index.bucket_index));
        val
    }

    /// Acquire an immutable reference to the `i`th element of the vector `v`.
    /// Aborts if `i` is out of bounds.
    public fun borrow<T>(v: &BigVector<T>, index: &BigVectorIndex): &T {
        vector::borrow(table_with_length::borrow(&v.buckets, index.bucket_index), index.vec_index)
    }

    /// Like `borrow()`, but accepts a `simple_index` corresponding the
    /// `i`th element of the entire `BigVector` indicated by `v`.
    ///
    /// Ideally a reference to the internally calculated
    /// `BigVectorIndex` would be simply passed to `borrow()`, but this
    /// approach violates the borrow checker. Hence the corresponding
    /// internal vector borrow is re-implemented per below.
    public fun borrow_simple<T>(
        v: &BigVector<T>,
        simple_index: u64
    ): &T {
        // Get big vector index
        let big_vector_index = bucket_index(v, simple_index);
        vector::borrow( // Immutably borrow corresponding element
            table_with_length::borrow(
                &v.buckets,
                big_vector_index.bucket_index
            ),
            big_vector_index.vec_index
        )
    }

    /// Return a mutable reference to the `i`th element in the vector `v`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(v: &mut BigVector<T>, index: &BigVectorIndex): &mut T {
        vector::borrow_mut(table_with_length::borrow_mut(&mut v.buckets, index.bucket_index), index.vec_index)
    }

    /// Wrapped version of `borrow_mut()`, accepting a `simple_index`
    /// corresponding to the `i`th element of the entire `BigVector`
    /// indicated by `v`.
    public fun borrow_simple_mut<T>(
        v: &mut BigVector<T>,
        simple_index: u64
    ): &mut T {
        // Get big vector index
        let big_vector_index = bucket_index(v, simple_index);
        // Mutably borrow corresponding element
        borrow_mut(v, &big_vector_index)
    }

    /// Return the length of the vector.
    public fun length<T>(v: &BigVector<T>): u64 {
        v.end_index.bucket_index * v.bucket_size + v.end_index.vec_index
    }

    /// Return `true` if the vector `v` has no elements and `false` otherwise.
    public fun is_empty<T>(v: &BigVector<T>): bool {
        length(v) == 0
    }

    /// Swap the `i`th element of the vector `v` with the last element and then pop the vector.
    /// This is O(1), but does not preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<T>(v: &mut BigVector<T>, index: &BigVectorIndex): T {
        let last_val = pop_back(v);
        // if the requested value is the last one, return it
        if (v.end_index.bucket_index == index.bucket_index && v.end_index.vec_index == index.vec_index) {
            return last_val
        };
        // because the lack of mem::swap, here we swap remove the requested value from the bucket
        // and append the last_val to the bucket then swap the last bucket val back
        let bucket = table_with_length::borrow_mut(&mut v.buckets, index.bucket_index);
        let bucket_len = vector::length(bucket);
        let val = vector::swap_remove(bucket, index.vec_index);
        vector::push_back(bucket, last_val);
        vector::swap(bucket, index.vec_index, bucket_len - 1);
        val
    }

    /// Wrapped version of `swap_remove()`, accepting a `simple_index`
    /// corresponding to the `i`th element of the entire `BigVector`
    /// indicated by `v`.
    public fun swap_remove_simple<T>(
        v: &mut BigVector<T>,
        simple_index: u64
    ): T {
        // Get big vector index
        let big_vector_index = bucket_index(v, simple_index);
        // Swap remove corresponding element
        swap_remove(v, &big_vector_index)
    }

    /// Return true if `val` is in the vector `v`.
    public fun contains<T>(v: &BigVector<T>, val: &T): bool {
        if (is_empty(v)) return false;
        let (exist, _) = index_of(v, val);
        exist
    }

    /// Return `(true, i)` if `val` is in the vector `v` at index `i`.
    /// Otherwise, returns `(false, 0)`.
    public fun index_of<T>(v: &BigVector<T>, val: &T): (bool, u64) {
        let i = 0;
        let len = length(v);
        let index = bucket_index(v, 0);
        while (i < len) {
            if (borrow(v, &index) == val) {
                return (true, i)
            };
            i = i + 1;
            increment_index(&mut index, v.bucket_size);
        };
        (false, 0)
    }

    /// Buckets related API

    /// Return corresponding BigVectorIndex for `i`, we can avoid this once table supports lookup by value instead of by reference.
    /// Aborts if `i` is out of bounds.
    public fun bucket_index<T>(v: &BigVector<T>, i: u64): BigVectorIndex {
        assert!(i < length(v), EINDEX_OUT_OF_BOUNDS);
        BigVectorIndex {
            bucket_index: i / v.bucket_size,
            vec_index: i % v.bucket_size,
        }
    }

    /// Return the bucket size of the vector.
    public fun bucket_size<T>(v: &BigVector<T>): u64 {
        v.bucket_size
    }

    /// Equivalent to i = i + 1 for BigVectorIndex with `bucket_size`.
    public fun increment_index(index: &mut BigVectorIndex, bucket_size: u64) {
        if (index.vec_index + 1 == bucket_size) {
            index.bucket_index  = index.bucket_index + 1;
            index.vec_index = 0;
        } else {
            index.vec_index = index.vec_index + 1;
        }
    }

    /// Equivalent to i = i - 1 for BigVectorIndex with `bucket_size`.
    /// Aborts if `i` becomes out of bounds.
    public fun decrement_index(index: &mut BigVectorIndex, bucket_size: u64) {
        if (index.vec_index == 0) {
            assert!(index.bucket_index > 0, EINDEX_OUT_OF_BOUNDS);
            index.bucket_index = index.bucket_index - 1;
            index.vec_index = bucket_size - 1;
        } else {
            index.vec_index = index.vec_index - 1;
        }
    }

    /// Reserve `additional_buckets` more buckets.
    public fun reserve<T>(v: &mut BigVector<T>, additional_buckets: u64) {
        while (additional_buckets > 0) {
            table_with_length::add(&mut v.buckets, v.num_buckets, vector::empty());
            v.num_buckets = v.num_buckets + 1;
            additional_buckets = additional_buckets - 1;
        }
    }

    /// Shrink the buckets to fit the current length.
    public fun shrink_to_fit<T>(v: &mut BigVector<T>) {
        while (v.num_buckets > buckets_required(&v.end_index)) {
            v.num_buckets = v.num_buckets - 1;
            let v = table_with_length::remove(&mut v.buckets, v.num_buckets);
            vector::destroy_empty(v);
        }
    }

    fun buckets_required(end_index: &BigVectorIndex): u64 {
        let additional = if (end_index.vec_index == 0) { 0 } else { 1 };
        end_index.bucket_index + additional
    }

    /// Return the number of elements of fixed-size type `T` that can
    /// fit into the optimal BLOB size, after allocating the necessary
    /// base size for a bucket vector.
    ///
    /// The underlying value indicated by `fixed_size_type_null_ref` is
    /// not required for calculations, and so it can be passed as a
    /// quasi-null value, e.g. `&false` for `T` as a `bool`, `&0` for
    /// `T` as a `u8`, and `&@0x0` for `T` as an `address`.
    ///
    /// Does not enforce that `T` actually has a fixed size, as the
    /// underlying `type_info::size_of()` implementation is incapable of
    /// such enforcement.
    ///
    /// See `test_get_optimal_bucket_size()` for examples.
    fun get_optimal_bucket_size<T: store>(
        fixed_size_type_null_ref: &T
    ): u64 {
        // Get element size, in bytes
        let element_size = type_info::size_of(fixed_size_type_null_ref);
        // Return optimal bucket size
        get_optimal_bucket_size_from_element_size(element_size)
    }

    /// Inner function for `get_optimal_bucket_size()`, isolated for
    /// simpler expected-failure unit testing. `element_size` is in
    /// bytes.
    fun get_optimal_bucket_size_from_element_size(
        element_size: u64
    ): u64 {
        // Calculate the maximum element size for an optimized bucket
        // when the underlying bucket vector only takes up one byte
        // beyond the total size of the fixed-size elements within
        let max_element_size_base_small =
            (OPTIMAL_BLOB_SIZE - VECTOR_BASE_SIZE_SMALL) /
                VECTOR_BASE_SIZE_LENGTH_CUTOFF;
        // The base size of the bucket vector is the smaller base size
        // when elements are larger than the cutoff, and the larger
        // base size when elements are smaller than the cutoff
        let vector_base_size = if (element_size > max_element_size_base_small)
            VECTOR_BASE_SIZE_SMALL else VECTOR_BASE_SIZE_LARGE;
        // Optimal bucket size is the number of elements that can fit
        // into the optimal BLOB size, after allocating the necessary
        // base size for the bucket vector
        let bucket_size =
            (OPTIMAL_BLOB_SIZE - vector_base_size) / element_size;
        // Assert that at least one element can fit in an optimal bucket
        assert!(bucket_size > 0, E_TOO_BIG_TO_OPTIMIZE);
        bucket_size // Return optimal bucket size
    }

    #[test]
    fun big_vector_test() {
        let v = new(5);
        let i = 0;
        while (i < 100) {
            push_back(&mut v, i);
            i = i + 1;
        };
        let j = 0;
        while (j < 100) {
            let index = bucket_index(&v, j);
            let val = borrow(&v, &index);
            assert!(*val == j, 0);
            j = j + 1;
        };
        while (i > 0) {
            i = i - 1;
            let (exist, index) = index_of(&v, &i);
            let j = pop_back(&mut v);
            assert!(exist, 0);
            assert!(index == i, 0);
            assert!(j == i, 0);
        };
        while (i < 100) {
            push_back(&mut v, i);
            i = i + 1;
        };
        let last_index = bucket_index(&v, length(&v) - 1);
        assert!(swap_remove(&mut v, &last_index) == 99, 0);
        let first_index = bucket_index(&v, 0);
        assert!(swap_remove(&mut v, &first_index) == 0, 0);
        while (length(&v) > 0) {
            // the vector is always [N, 1, 2, ... N-1] with repetitive swap_remove(&mut v, 0)
            let expected = length(&v);
            let index = bucket_index(&v, 0);
            let val = swap_remove(&mut v, &index);
            assert!(val == expected, 0);
        };
        shrink_to_fit(&mut v);
        destroy_empty(v);
    }

    #[test]
    #[expected_failure]
    fun big_vector_need_grow() {
        let v = new_with_capacity(5, 1);
        let i = 0;
        while (i < 6) {
            push_back_no_grow(&mut v, i);
            i = i + 1;
        };
        destroy_empty(v);
    }

    #[test]
    fun big_vector_reserve_and_shrink() {
        let v = new (10);
        reserve(&mut v, 10);
        assert!(v.num_buckets == 10, 0);
        let i = 0;
        while (i < 100) {
            push_back_no_grow(&mut v, i);
            i = i + 1;
        };
        while (i < 120) {
            push_back(&mut v, i);
            i = i + 1;
        };
        while (i > 90) {
            pop_back(&mut v);
            i = i - 1;
        };
        assert!(v.num_buckets == 12, 0);
        shrink_to_fit(&mut v);
        assert!(v.num_buckets == 9, 0);
        while (i > 55) {
            pop_back(&mut v);
            i = i - 1;
        };
        shrink_to_fit(&mut v);
        assert!(v.num_buckets == 6, 0);
        while (i > 0) {
            pop_back(&mut v);
            i = i - 1;
        };
        shrink_to_fit(&mut v);
        destroy_empty(v);
    }

    #[test]
    fun big_vector_empty_contains() {
        let v = new<u64> (10);
        assert!(!contains<u64>(&v, &(1 as u64)), 0);
        destroy_empty(v);
    }

    #[test]
    #[expected_failure(abort_code = 4)]
    /// Verify failure for an element that is too large
    fun test_get_optimal_bucket_size_from_element_size() {
        // Attempt invalid invocation with element that is 1 byte too
        // large
        get_optimal_bucket_size_from_element_size(OPTIMAL_BLOB_SIZE);
    }

    #[test]
    /// Verify expected returns for assorted sizes
    fun test_get_optimal_bucket_size() {
        assert!(get_optimal_bucket_size_from_element_size(4095) == 1, 0);
        assert!(get_optimal_bucket_size_from_element_size(1) == 4094, 0);
        assert!(get_optimal_bucket_size(&false) == 4094, 0);
        assert!(get_optimal_bucket_size<u8>(&0) == 4094, 0);
        assert!(get_optimal_bucket_size_from_element_size(8) == 511, 0);
        assert!(get_optimal_bucket_size<u64>(&0) == 511, 0);
        assert!(get_optimal_bucket_size_from_element_size(16) == 255, 0);
        assert!(get_optimal_bucket_size<u128>(&0) == 255, 0);
        assert!(get_optimal_bucket_size_from_element_size(32) == 127, 0);
        assert!(get_optimal_bucket_size(&@0x0) == 127, 0);
        // Below cases exercise logic associated with the size length
        // cutoff: at a vector base size of 1 byte, 4095 bytes are
        // reserved for elements. 13 is a prime factor of 4095, such
        // that 105 elements of size 39 can fit perfectly:
        assert!(get_optimal_bucket_size_from_element_size(39) == 105, 0);
        // The max element size while still having a vector base size of
        // 1 byte is 32 bytes (after truncating division), hence
        // elements with a size of 33 result in the lower vector base
        // size
        assert!(get_optimal_bucket_size_from_element_size(33) == 124, 0);
        // An element size of 32 is the cutoff.
        assert!(get_optimal_bucket_size_from_element_size(32) == 127, 0);
        // An element size of 31 is just under the cutoff, hence there
        // are only 4094 bytes reserved for elements, in which 132 may
        // fit
        assert!(get_optimal_bucket_size_from_element_size(31) == 132, 0);
        // Here, 4094 bytes are reserved for elements, and 23 is a prime
        // factor of 23. Hence 178 elements fit perfectly
        assert!(get_optimal_bucket_size_from_element_size(23) == 178, 0);
    }

    #[test]
    /// Verify expected initialization
    fun test_new_optimized() {
        // Declare empty vector for u128 elements
        let empty_vector = new_optimized<u128>(&0);
        // Assert expected bucket size
        assert!(empty_vector.bucket_size == 255, 0);
        destroy_empty(empty_vector); // Destroy empty vector
    }

    #[test_only]
    struct TestStruct<T> has copy, drop, store {
        field_1: u64,
        field_2: T
    }

    #[test]
    /// Verify expected initialization
    fun test_new_with_capacity_optimized() {
        let num_buckets = 25; // Declare number of buckets to init
        // Declare empty vector for address elements
        let empty_vector = new_with_capacity_optimized(&TestStruct{
            field_1: 0, field_2: @0x0}, num_buckets);
        // Calculate expected bucket size
        let bucket_size_expected = (OPTIMAL_BLOB_SIZE - VECTOR_BASE_SIZE_SMALL)
            / (8 + 32); // u64 is 8 bytes, address is 32
        // Assert expected bucket size
        assert!(empty_vector.bucket_size == bucket_size_expected, 0);
        destroy_empty(empty_vector); // Destroy empty vector
    }

    #[test]
    /// Verify expected initialization
    fun test_singleton_optimized():
    BigVector<u64> {
        let element = 25; // Declare singleton element
        // Declare empty vector for u128 elements
        let singleton = singleton_optimized(element);
        // Assert expected bucket size
        assert!(singleton.bucket_size == 511, 0);
        // Assert first element is as expected
        assert!(*borrow_simple(&singleton, 0) == element, 0);
        singleton // Return rather than unpack
    }

    #[test]
    /// Verify expected initialization
    fun test_singleton_with_capacity_optimized():
    BigVector<TestStruct<bool>> {
        // Declare singleton element
        let element = TestStruct{field_1: 0, field_2: false};
        let num_buckets = 35; // Declare number of buckets to init
        // Declare empty vector with TestStruct of type bool
        let singleton =
            singleton_with_capacity_optimized(element, num_buckets);
        // Calculate expected bucket size
        let bucket_size_expected =
            // u64 is 8 bytes, bool takes 1
            (OPTIMAL_BLOB_SIZE - VECTOR_BASE_SIZE_LARGE) / (8 + 1);
        // Assert expected bucket size
        assert!(singleton.bucket_size == bucket_size_expected, 0);
        // Assert first element is as expected
        //assert!(*borrow_simple(&singleton, 0) == element, 0);
        singleton // Return rather than unpack
    }

    #[test]
    /// Verify expected mutations, state lookups, for functions that
    /// accept simple indices
    fun test_simple_index_operations() {
        // Define element values
        let (e_0, e_1, e_1_prime, e_2) = (1, 2, 3, 4);
        // Create a singleton vector, then append additional elements
        let big_vector = singleton_optimized<u8>(e_0);
        push_back(&mut big_vector, e_1);
        push_back(&mut big_vector, e_2);
        // Assert element at index 1 pre-mutation
        assert!(*borrow_simple(&big_vector, 1) == e_1, 0);
        // Mutate element at index 1
        *borrow_simple_mut(&mut big_vector, 1) = e_1_prime;
        // Assert element at index 1 post-mutation
        assert!(*borrow_simple(&big_vector, 1) == e_1_prime, 0);
        // Swap remove element at index 0
        assert!(swap_remove_simple(&mut big_vector, 0) == e_0, 0);
        // Swap remove new element at index 0
        assert!(swap_remove_simple(&mut big_vector, 0) == e_2, 0);
        // Swap remove only remaining element
        assert!(swap_remove_simple(&mut big_vector, 0) == e_1_prime, 0);
        destroy_empty(big_vector) // Destroy empty vector
    }

}
