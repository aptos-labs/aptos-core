module aptos_std::big_vector {
    use std::error;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};

    /// Vector index is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 1;
    /// Vector is full
    const EOUT_OF_CAPACITY: u64 = 2;
    /// Cannot destroy a non-empty vector
    const EVECTOR_NOT_EMPTY: u64 = 3;

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

    /// Return a mutable reference to the `i`th element in the vector `v`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(v: &mut BigVector<T>, index: &BigVectorIndex): &mut T {
        vector::borrow_mut(table_with_length::borrow_mut(&mut v.buckets, index.bucket_index), index.vec_index)
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
}
