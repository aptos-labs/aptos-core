module aptos_std::big_vector {
    use std::error;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};
    friend aptos_std::smart_vector;

    /// Vector index is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 1;
    /// Cannot destroy a non-empty vector
    const EVECTOR_NOT_EMPTY: u64 = 2;
    /// Cannot pop back from an empty vector
    const EVECTOR_EMPTY: u64 = 3;
    /// bucket_size cannot be 0
    const EZERO_BUCKET_SIZE: u64 = 4;

    /// A scalable vector implementation based on tables where elements are grouped into buckets.
    /// Each bucket has a capacity of `bucket_size` elements.
    struct BigVector<T> has store {
        buckets: TableWithLength<u64, vector<T>>,
        end_index: u64,
        bucket_size: u64
    }

    /// Regular Vector API

    /// Create an empty vector.
    public(friend) fun empty<T: store>(bucket_size: u64): BigVector<T> {
        assert!(bucket_size > 0, error::invalid_argument(EZERO_BUCKET_SIZE));
        BigVector {
            buckets: table_with_length::new(),
            end_index: 0,
            bucket_size,
        }
    }

    /// Create a vector of length 1 containing the passed in element.
    public(friend) fun singleton<T: store>(element: T, bucket_size: u64): BigVector<T> {
        let v = empty(bucket_size);
        push_back(&mut v, element);
        v
    }

    /// Destroy the vector `self`.
    /// Aborts if `self` is not empty.
    public fun destroy_empty<T>(self: BigVector<T>) {
        assert!(is_empty(&self), error::invalid_argument(EVECTOR_NOT_EMPTY));
        let BigVector { buckets, end_index: _, bucket_size: _ } = self;
        table_with_length::destroy_empty(buckets);
    }

    /// Destroy the vector `self` if T has `drop`
    public fun destroy<T: drop>(self: BigVector<T>) {
        let BigVector { buckets, end_index, bucket_size: _ } = self;
        let i = 0;
        while (end_index > 0) {
            let num_elements = vector::length(&table_with_length::remove(&mut buckets, i));
            end_index = end_index - num_elements;
            i = i + 1;
        };
        table_with_length::destroy_empty(buckets);
    }

    /// Acquire an immutable reference to the `i`th element of the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow<T>(self: &BigVector<T>, i: u64): &T {
        assert!(i < length(self), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        vector::borrow(table_with_length::borrow(&self.buckets, i / self.bucket_size), i % self.bucket_size)
    }

    /// Return a mutable reference to the `i`th element in the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(self: &mut BigVector<T>, i: u64): &mut T {
        assert!(i < length(self), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        vector::borrow_mut(table_with_length::borrow_mut(&mut self.buckets, i / self.bucket_size), i % self.bucket_size)
    }

    /// Empty and destroy the other vector, and push each of the elements in the other vector onto the self vector in the
    /// same order as they occurred in other.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun append<T: store>(self: &mut BigVector<T>, other: BigVector<T>) {
        let other_len = length(&other);
        let half_other_len = other_len / 2;
        let i = 0;
        while (i < half_other_len) {
            push_back(self, swap_remove(&mut other, i));
            i = i + 1;
        };
        while (i < other_len) {
            push_back(self, pop_back(&mut other));
            i = i + 1;
        };
        destroy_empty(other);
    }

    /// Add element `val` to the end of the vector `self`. It grows the buckets when the current buckets are full.
    /// This operation will cost more gas when it adds new bucket.
    public fun push_back<T: store>(self: &mut BigVector<T>, val: T) {
        let num_buckets = table_with_length::length(&self.buckets);
        if (self.end_index == num_buckets * self.bucket_size) {
            table_with_length::add(&mut self.buckets, num_buckets, vector::empty());
            vector::push_back(table_with_length::borrow_mut(&mut self.buckets, num_buckets), val);
        } else {
            vector::push_back(table_with_length::borrow_mut(&mut self.buckets, num_buckets - 1), val);
        };
        self.end_index = self.end_index + 1;
    }

    /// Pop an element from the end of vector `self`. It doesn't shrink the buckets even if they're empty.
    /// Call `shrink_to_fit` explicity to deallocate empty buckets.
    /// Aborts if `self` is empty.
    public fun pop_back<T>(self: &mut BigVector<T>): T {
        assert!(!is_empty(self), error::invalid_state(EVECTOR_EMPTY));
        let num_buckets = table_with_length::length(&self.buckets);
        let last_bucket = table_with_length::borrow_mut(&mut self.buckets, num_buckets - 1);
        let val = vector::pop_back(last_bucket);
        // Shrink the table if the last vector is empty.
        if (vector::is_empty(last_bucket)) {
            move last_bucket;
            vector::destroy_empty(table_with_length::remove(&mut self.buckets, num_buckets - 1));
        };
        self.end_index = self.end_index - 1;
        val
    }

    /// Remove the element at index i in the vector v and return the owned value that was previously stored at i in self.
    /// All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun remove<T>(self: &mut BigVector<T>, i: u64): T {
        let len = length(self);
        assert!(i < len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let num_buckets = table_with_length::length(&self.buckets);
        let cur_bucket_index = i / self.bucket_size + 1;
        let cur_bucket = table_with_length::borrow_mut(&mut self.buckets, cur_bucket_index - 1);
        let res = vector::remove(cur_bucket, i % self.bucket_size);
        self.end_index = self.end_index - 1;
        move cur_bucket;
        while ({
            spec {
                invariant cur_bucket_index <= num_buckets;
                invariant table_with_length::spec_len(self.buckets) == num_buckets;
            };
            (cur_bucket_index < num_buckets)
        }) {
            // remove one element from the start of current vector
            let cur_bucket = table_with_length::borrow_mut(&mut self.buckets, cur_bucket_index);
            let t = vector::remove(cur_bucket, 0);
            move cur_bucket;
            // and put it at the end of the last one
            let prev_bucket = table_with_length::borrow_mut(&mut self.buckets, cur_bucket_index - 1);
            vector::push_back(prev_bucket, t);
            cur_bucket_index = cur_bucket_index + 1;
        };
        spec {
            assert cur_bucket_index == num_buckets;
        };

        // Shrink the table if the last vector is empty.
        let last_bucket = table_with_length::borrow_mut(&mut self.buckets, num_buckets - 1);
        if (vector::is_empty(last_bucket)) {
            move last_bucket;
            vector::destroy_empty(table_with_length::remove(&mut self.buckets, num_buckets - 1));
        };

        res
    }

    /// Swap the `i`th element of the vector `self` with the last element and then pop the vector.
    /// This is O(1), but does not preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<T>(self: &mut BigVector<T>, i: u64): T {
        assert!(i < length(self), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let last_val = pop_back(self);
        // if the requested value is the last one, return it
        if (self.end_index == i) {
            return last_val
        };
        // because the lack of mem::swap, here we swap remove the requested value from the bucket
        // and append the last_val to the bucket then swap the last bucket val back
        let bucket = table_with_length::borrow_mut(&mut self.buckets, i / self.bucket_size);
        let bucket_len = vector::length(bucket);
        let val = vector::swap_remove(bucket, i % self.bucket_size);
        vector::push_back(bucket, last_val);
        vector::swap(bucket, i % self.bucket_size, bucket_len - 1);
        val
    }

    /// Swap the elements at the i'th and j'th indices in the vector self. Will abort if either of i or j are out of bounds
    /// for self.
    public fun swap<T>(self: &mut BigVector<T>, i: u64, j: u64) {
        assert!(i < length(self) && j < length(self), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let i_bucket_index = i / self.bucket_size;
        let j_bucket_index = j / self.bucket_size;
        let i_vector_index = i % self.bucket_size;
        let j_vector_index = j % self.bucket_size;
        if (i_bucket_index == j_bucket_index) {
            vector::swap(table_with_length::borrow_mut(&mut self.buckets, i_bucket_index), i_vector_index, j_vector_index);
            return
        };
        // If i and j are in different buckets, take the buckets out first for easy mutation.
        let bucket_i = table_with_length::remove(&mut self.buckets, i_bucket_index);
        let bucket_j = table_with_length::remove(&mut self.buckets, j_bucket_index);
        // Get the elements from buckets by calling `swap_remove`.
        let element_i = vector::swap_remove(&mut bucket_i, i_vector_index);
        let element_j = vector::swap_remove(&mut bucket_j, j_vector_index);
        // Swap the elements and push back to the other bucket.
        vector::push_back(&mut bucket_i, element_j);
        vector::push_back(&mut bucket_j, element_i);
        let last_index_in_bucket_i = vector::length(&bucket_i) - 1;
        let last_index_in_bucket_j = vector::length(&bucket_j) - 1;
        // Re-position the swapped elements to the right index.
        vector::swap(&mut bucket_i, i_vector_index, last_index_in_bucket_i);
        vector::swap(&mut bucket_j, j_vector_index, last_index_in_bucket_j);
        // Add back the buckets.
        table_with_length::add(&mut self.buckets, i_bucket_index, bucket_i);
        table_with_length::add(&mut self.buckets, j_bucket_index, bucket_j);
    }

    /// Reverse the order of the elements in the vector self in-place.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun reverse<T>(self: &mut BigVector<T>) {
        let new_buckets = vector[];
        let push_bucket = vector[];
        let num_buckets = table_with_length::length(&self.buckets);
        let num_buckets_left = num_buckets;

        while (num_buckets_left > 0) {
            let pop_bucket = table_with_length::remove(&mut self.buckets, num_buckets_left - 1);
            vector::for_each_reverse(pop_bucket, |val| {
                vector::push_back(&mut push_bucket, val);
                if (vector::length(&push_bucket) == self.bucket_size) {
                    vector::push_back(&mut new_buckets, push_bucket);
                    push_bucket = vector[];
                };
            });
            num_buckets_left = num_buckets_left - 1;
        };

        if (vector::length(&push_bucket) > 0) {
            vector::push_back(&mut new_buckets, push_bucket);
        } else {
            vector::destroy_empty(push_bucket);
        };

        vector::reverse(&mut new_buckets);
        let i = 0;
        assert!(table_with_length::length(&self.buckets) == 0, 0);
        while (i < num_buckets) {
            table_with_length::add(&mut self.buckets, i, vector::pop_back(&mut new_buckets));
            i = i + 1;
        };
        vector::destroy_empty(new_buckets);
    }

    /// Return the index of the first occurrence of an element in self that is equal to e. Returns (true, index) if such an
    /// element was found, and (false, 0) otherwise.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun index_of<T>(self: &BigVector<T>, val: &T): (bool, u64) {
        let num_buckets = table_with_length::length(&self.buckets);
        let bucket_index = 0;
        while (bucket_index < num_buckets) {
            let cur = table_with_length::borrow(&self.buckets, bucket_index);
            let (found, i) = vector::index_of(cur, val);
            if (found) {
                return (true, bucket_index * self.bucket_size + i)
            };
            bucket_index = bucket_index + 1;
        };
        (false, 0)
    }

    /// Return if an element equal to e exists in the vector self.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun contains<T>(self: &BigVector<T>, val: &T): bool {
        if (is_empty(self)) return false;
        let (exist, _) = index_of(self, val);
        exist
    }

    /// Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an
    /// atomic view of the whole vector.
    /// Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.
    public fun to_vector<T: copy>(self: &BigVector<T>): vector<T> {
        let res = vector[];
        let num_buckets = table_with_length::length(&self.buckets);
        let i = 0;
        while (i < num_buckets) {
            vector::append(&mut res, *table_with_length::borrow(&self.buckets, i));
            i = i + 1;
        };
        res
    }

    /// Return the length of the vector.
    public fun length<T>(self: &BigVector<T>): u64 {
        self.end_index
    }

    /// Return `true` if the vector `v` has no elements and `false` otherwise.
    public fun is_empty<T>(self: &BigVector<T>): bool {
        length(self) == 0
    }

    #[test]
    fun big_vector_test() {
        let v = empty(5);
        let i = 0;
        while (i < 100) {
            push_back(&mut v, i);
            i = i + 1;
        };
        let j = 0;
        while (j < 100) {
            let val = borrow(&v, j);
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
        let last_index = length(&v) - 1;
        assert!(swap_remove(&mut v, last_index) == 99, 0);
        assert!(swap_remove(&mut v, 0) == 0, 0);
        while (length(&v) > 0) {
            // the vector is always [N, 1, 2, ... N-1] with repetitive swap_remove(&mut v, 0)
            let expected = length(&v);
            let val = swap_remove(&mut v, 0);
            assert!(val == expected, 0);
        };
        destroy_empty(v);
    }

    #[test]
    fun big_vector_append_edge_case_test() {
        let v1 = empty(5);
        let v2 = singleton(1u64, 7);
        let v3 = empty(6);
        let v4 = empty(8);
        append(&mut v3, v4);
        assert!(length(&v3) == 0, 0);
        append(&mut v2, v3);
        assert!(length(&v2) == 1, 0);
        append(&mut v1, v2);
        assert!(length(&v1) == 1, 0);
        destroy(v1);
    }

    #[test]
    fun big_vector_append_test() {
        let v1 = empty(5);
        let v2 = empty(7);
        let i = 0;
        while (i < 7) {
            push_back(&mut v1, i);
            i = i + 1;
        };
        while (i < 25) {
            push_back(&mut v2, i);
            i = i + 1;
        };
        append(&mut v1, v2);
        assert!(length(&v1) == 25, 0);
        i = 0;
        while (i < 25) {
            assert!(*borrow(&v1, i) == i, 0);
            i = i + 1;
        };
        destroy(v1);
    }

    #[test]
    fun big_vector_to_vector_test() {
        let v1 = empty(7);
        let i = 0;
        while (i < 100) {
            push_back(&mut v1, i);
            i = i + 1;
        };
        let v2 = to_vector(&v1);
        let j = 0;
        while (j < 100) {
            assert!(*vector::borrow(&v2, j) == j, 0);
            j = j + 1;
        };
        destroy(v1);
    }

    #[test]
    fun big_vector_remove_and_reverse_test() {
        let v = empty(11);
        let i = 0;
        while (i < 101) {
            push_back(&mut v, i);
            i = i + 1;
        };
        remove(&mut v, 100);
        remove(&mut v, 90);
        remove(&mut v, 80);
        remove(&mut v, 70);
        remove(&mut v, 60);
        remove(&mut v, 50);
        remove(&mut v, 40);
        remove(&mut v, 30);
        remove(&mut v, 20);
        remove(&mut v, 10);
        remove(&mut v, 0);
        assert!(length(&v) == 90, 0);

        let index = 0;
        i = 0;
        while (i < 101) {
            if (i % 10 != 0) {
                assert!(*borrow(&v, index) == i, 0);
                index = index + 1;
            };
            i = i + 1;
        };
        destroy(v);
    }

    #[test]
    fun big_vector_swap_test() {
        let v = empty(11);
        let i = 0;
        while (i < 101) {
            push_back(&mut v, i);
            i = i + 1;
        };
        i = 0;
        while (i < 51) {
            swap(&mut v, i, 100 - i);
            i = i + 1;
        };
        i = 0;
        while (i < 101) {
            assert!(*borrow(&v, i) == 100 - i, 0);
            i = i + 1;
        };
        destroy(v);
    }

    #[test]
    fun big_vector_index_of_test() {
        let v = empty(11);
        let i = 0;
        while (i < 100) {
            push_back(&mut v, i);
            let (found, idx) = index_of(&mut v, &i);
            assert!(found && idx == i, 0);
            i = i + 1;
        };
        destroy(v);
    }

    #[test]
    fun big_vector_empty_contains() {
        let v = empty<u64>(10);
        assert!(!contains<u64>(&v, &(1 as u64)), 0);
        destroy_empty(v);
    }
}
