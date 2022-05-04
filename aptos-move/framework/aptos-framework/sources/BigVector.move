module AptosFramework::BigVector {
    use Std::Vector;
    use AptosFramework::Table::{Self, Table};

    /// The index into the vector is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 0;

    /// Index of the value in the buckets.
    struct BigVectorIndex has copy, drop, store {
        bucket_index: u64,
        vec_index: u64,
    }

    /// A Scalable vector implementation based on tables, elements are grouped into buckets with `bucket_size`.
    struct BigVector<T> has store {
        buckets: Table<u64, vector<T>>,
        next_index: BigVectorIndex,
        bucket_size: u64
    }

    /// Regular Vector API

    /// Create an empty vector.
    public fun new<T: store>(bucket_size: u64): BigVector<T> {
        assert!(bucket_size > 0, 0);
        BigVector {
            buckets: Table::new(),
            next_index: BigVectorIndex {
                bucket_index: 0,
                vec_index: 0,
            },
            bucket_size,
        }
    }

    /// Destroy the vector `v`.
    /// Aborts if `v` is not empty.
    public fun destroy_empty<T>(v: BigVector<T>) {
        assert!(is_empty(&v), 0);
        let BigVector { buckets, next_index: _, bucket_size: _ } = v;
        Table::destroy_empty(buckets);
    }

    /// Add element `e` to the end of the vector `v`.
    public fun push_back<T>(v: &mut BigVector<T>, val: T) {
        if (v.next_index.vec_index == 0) {
            Table::add(&mut v.buckets, &v.next_index.bucket_index, Vector::empty());
        };
        Vector::push_back(Table::borrow_mut(&mut v.buckets, &v.next_index.bucket_index), val);
        increment_index(&mut v.next_index, v.bucket_size);
    }

    /// Pop an element from the end of vector `v`.
    /// Aborts if `v` is empty.
    public fun pop_back<T>(v: &mut BigVector<T>): T {
        assert!(!is_empty(v), EINDEX_OUT_OF_BOUNDS);
        decrement_index(&mut v.next_index, v.bucket_size);
        let val = Vector::pop_back(Table::borrow_mut(&mut v.buckets, &v.next_index.bucket_index));
        if (v.next_index.vec_index == 0) {
            Vector::destroy_empty(Table::remove(&mut v.buckets, &v.next_index.bucket_index));
        };
        val
    }

    /// Acquire an immutable reference to the `i`th element of the vector `v`.
    /// Aborts if `i` is out of bounds.
    public fun borrow<T>(v: &BigVector<T>, index: &BigVectorIndex): &T {
        Vector::borrow(Table::borrow(&v.buckets, &index.bucket_index), index.vec_index)
    }

    /// Return a mutable reference to the `i`th element in the vector `v`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(v: &mut BigVector<T>, index: &BigVectorIndex): &mut T {
        Vector::borrow_mut(Table::borrow_mut(&mut v.buckets, &index.bucket_index), index.vec_index)
    }

    /// Return the length of the vector.
    public fun length<T>(v: &BigVector<T>): u64 {
        v.next_index.bucket_index * v.bucket_size + v.next_index.vec_index
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
        if (v.next_index.bucket_index == index.bucket_index && v.next_index.vec_index == index.vec_index) {
            return last_val
        };
        // because the lack of mem::swap, here we swap remove the requested value from the bucket
        // and append the last_val to the bucket then swap the last bucket val back
        let bucket = Table::borrow_mut(&mut v.buckets, &index.bucket_index);
        let bucket_len = Vector::length(bucket);
        let val = Vector::swap_remove(bucket, index.vec_index);
        Vector::push_back(bucket, last_val);
        Vector::swap(bucket, index.vec_index, bucket_len - 1);
        val
    }

    /// Return true if `e` is in the vector `v`.
    public fun contains<T>(v: &BigVector<T>, val: &T): bool {
        let (exist, _) = index_of(v, val);
        exist
    }

    /// Return `(true, i)` if `e` is in the vector `v` at index `i`.
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
        destroy_empty(v);
    }
}
