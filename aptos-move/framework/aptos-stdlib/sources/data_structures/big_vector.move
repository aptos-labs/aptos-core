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
    friend fun empty<T: store>(bucket_size: u64): BigVector<T> {
        assert!(bucket_size > 0, error::invalid_argument(EZERO_BUCKET_SIZE));
        BigVector {
            buckets: table_with_length::new(),
            end_index: 0,
            bucket_size,
        }
    }

    /// Create a vector of length 1 containing the passed in element.
    friend fun singleton<T: store>(element: T, bucket_size: u64): BigVector<T> {
        let v = empty(bucket_size);
        v.push_back(element);
        v
    }

    /// Destroy the vector `self`.
    /// Aborts if `self` is not empty.
    public fun destroy_empty<T>(self: BigVector<T>) {
        assert!(self.is_empty(), error::invalid_argument(EVECTOR_NOT_EMPTY));
        let BigVector { buckets, end_index: _, bucket_size: _ } = self;
        buckets.destroy_empty();
    }

    /// Destroy the vector `self` if T has `drop`
    public fun destroy<T: drop>(self: BigVector<T>) {
        let BigVector { buckets, end_index, bucket_size: _ } = self;
        let i = 0;
        while (end_index > 0) {
            let num_elements = buckets.remove(i).length();
            end_index -= num_elements;
            i += 1;
        };
        buckets.destroy_empty();
    }

    /// Acquire an immutable reference to the `i`th element of the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow<T>(self: &BigVector<T>, i: u64): &T {
        assert!(i < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        self.buckets.borrow(i / self.bucket_size).borrow(i % self.bucket_size)
    }

    /// Return a mutable reference to the `i`th element in the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(self: &mut BigVector<T>, i: u64): &mut T {
        assert!(i < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        self.buckets.borrow_mut(i / self.bucket_size).borrow_mut(i % self.bucket_size)
    }

    /// Empty and destroy the other vector, and push each of the elements in the other vector onto the self vector in the
    /// same order as they occurred in other.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun append<T: store>(self: &mut BigVector<T>, other: BigVector<T>) {
        let other_len = other.length();
        let half_other_len = other_len / 2;
        let i = 0;
        while (i < half_other_len) {
            self.push_back(other.swap_remove(i));
            i += 1;
        };
        while (i < other_len) {
            self.push_back(other.pop_back());
            i += 1;
        };
        other.destroy_empty();
    }

    /// Add element `val` to the end of the vector `self`. It grows the buckets when the current buckets are full.
    /// This operation will cost more gas when it adds new bucket.
    public fun push_back<T: store>(self: &mut BigVector<T>, val: T) {
        let num_buckets = self.buckets.length();
        if (self.end_index == num_buckets * self.bucket_size) {
            self.buckets.add(num_buckets, vector::empty());
            self.buckets.borrow_mut(num_buckets).push_back(val);
        } else {
            self.buckets.borrow_mut(num_buckets - 1).push_back(val);
        };
        self.end_index += 1;
    }

    /// Pop an element from the end of vector `self`. It doesn't shrink the buckets even if they're empty.
    /// Call `shrink_to_fit` explicity to deallocate empty buckets.
    /// Aborts if `self` is empty.
    public fun pop_back<T>(self: &mut BigVector<T>): T {
        assert!(!self.is_empty(), error::invalid_state(EVECTOR_EMPTY));
        let num_buckets = self.buckets.length();
        let last_bucket = self.buckets.borrow_mut(num_buckets - 1);
        let val = last_bucket.pop_back();
        // Shrink the table if the last vector is empty.
        if (last_bucket.is_empty()) {
            move last_bucket;
            self.buckets.remove(num_buckets - 1).destroy_empty();
        };
        self.end_index -= 1;
        val
    }

    /// Remove the element at index i in the vector v and return the owned value that was previously stored at i in self.
    /// All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun remove<T>(self: &mut BigVector<T>, i: u64): T {
        let len = self.length();
        assert!(i < len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let num_buckets = self.buckets.length();
        let cur_bucket_index = i / self.bucket_size + 1;
        let cur_bucket = self.buckets.borrow_mut(cur_bucket_index - 1);
        let res = cur_bucket.remove(i % self.bucket_size);
        self.end_index -= 1;
        move cur_bucket;
        while ({
            spec {
                invariant cur_bucket_index <= num_buckets;
                invariant table_with_length::spec_len(self.buckets) == num_buckets;
            };
            (cur_bucket_index < num_buckets)
        }) {
            // remove one element from the start of current vector
            let cur_bucket = self.buckets.borrow_mut(cur_bucket_index);
            let t = cur_bucket.remove(0);
            move cur_bucket;
            // and put it at the end of the last one
            let prev_bucket = self.buckets.borrow_mut(cur_bucket_index - 1);
            prev_bucket.push_back(t);
            cur_bucket_index += 1;
        };
        spec {
            assert cur_bucket_index == num_buckets;
        };

        // Shrink the table if the last vector is empty.
        let last_bucket = self.buckets.borrow_mut(num_buckets - 1);
        if (last_bucket.is_empty()) {
            move last_bucket;
            self.buckets.remove(num_buckets - 1).destroy_empty();
        };

        res
    }

    /// Swap the `i`th element of the vector `self` with the last element and then pop the vector.
    /// This is O(1), but does not preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<T>(self: &mut BigVector<T>, i: u64): T {
        assert!(i < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let last_val = self.pop_back();
        // if the requested value is the last one, return it
        if (self.end_index == i) {
            return last_val
        };
        // because the lack of mem::swap, here we swap remove the requested value from the bucket
        // and append the last_val to the bucket then swap the last bucket val back
        let bucket = self.buckets.borrow_mut(i / self.bucket_size);
        let bucket_len = bucket.length();
        let val = bucket.swap_remove(i % self.bucket_size);
        bucket.push_back(last_val);
        bucket.swap(i % self.bucket_size, bucket_len - 1);
        val
    }

    /// Swap the elements at the i'th and j'th indices in the vector self. Will abort if either of i or j are out of bounds
    /// for self.
    public fun swap<T>(self: &mut BigVector<T>, i: u64, j: u64) {
        assert!(i < self.length() && j < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let i_bucket_index = i / self.bucket_size;
        let j_bucket_index = j / self.bucket_size;
        let i_vector_index = i % self.bucket_size;
        let j_vector_index = j % self.bucket_size;
        if (i_bucket_index == j_bucket_index) {
            self.buckets.borrow_mut(i_bucket_index).swap(i_vector_index, j_vector_index);
            return
        };
        // If i and j are in different buckets, take the buckets out first for easy mutation.
        let bucket_i = self.buckets.remove(i_bucket_index);
        let bucket_j = self.buckets.remove(j_bucket_index);
        // Get the elements from buckets by calling `swap_remove`.
        let element_i = bucket_i.swap_remove(i_vector_index);
        let element_j = bucket_j.swap_remove(j_vector_index);
        // Swap the elements and push back to the other bucket.
        bucket_i.push_back(element_j);
        bucket_j.push_back(element_i);
        let last_index_in_bucket_i = bucket_i.length() - 1;
        let last_index_in_bucket_j = bucket_j.length() - 1;
        // Re-position the swapped elements to the right index.
        bucket_i.swap(i_vector_index, last_index_in_bucket_i);
        bucket_j.swap(j_vector_index, last_index_in_bucket_j);
        // Add back the buckets.
        self.buckets.add(i_bucket_index, bucket_i);
        self.buckets.add(j_bucket_index, bucket_j);
    }

    /// Reverse the order of the elements in the vector self in-place.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun reverse<T>(self: &mut BigVector<T>) {
        let new_buckets = vector[];
        let push_bucket = vector[];
        let num_buckets = self.buckets.length();
        let num_buckets_left = num_buckets;

        while (num_buckets_left > 0) {
            let pop_bucket = self.buckets.remove(num_buckets_left - 1);
            pop_bucket.for_each_reverse(|val| {
                push_bucket.push_back(val);
                if (push_bucket.length() == self.bucket_size) {
                    new_buckets.push_back(push_bucket);
                    push_bucket = vector[];
                };
            });
            num_buckets_left -= 1;
        };

        if (push_bucket.length() > 0) {
            new_buckets.push_back(push_bucket);
        } else {
            push_bucket.destroy_empty();
        };

        new_buckets.reverse();
        assert!(self.buckets.length() == 0, 0);
        for (i in 0..num_buckets) {
            self.buckets.add(i, new_buckets.pop_back());
        };
        new_buckets.destroy_empty();
    }

    /// Return the index of the first occurrence of an element in self that is equal to e. Returns (true, index) if such an
    /// element was found, and (false, 0) otherwise.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun index_of<T>(self: &BigVector<T>, val: &T): (bool, u64) {
        let num_buckets = self.buckets.length();
        let bucket_index = 0;
        while (bucket_index < num_buckets) {
            let cur = self.buckets.borrow(bucket_index);
            let (found, i) = cur.index_of(val);
            if (found) {
                return (true, bucket_index * self.bucket_size + i)
            };
            bucket_index += 1;
        };
        (false, 0)
    }

    /// Return if an element equal to e exists in the vector self.
    /// Disclaimer: This function is costly. Use it at your own discretion.
    public fun contains<T>(self: &BigVector<T>, val: &T): bool {
        if (self.is_empty()) return false;
        let (exist, _) = self.index_of(val);
        exist
    }

    /// Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an
    /// atomic view of the whole vector.
    /// Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.
    public fun to_vector<T: copy>(self: &BigVector<T>): vector<T> {
        let res = vector[];
        let num_buckets = self.buckets.length();
        for (i in 0..num_buckets) {
            res.append(*self.buckets.borrow(i));
        };
        res
    }

    /// Return the length of the vector.
    public fun length<T>(self: &BigVector<T>): u64 {
        self.end_index
    }

    /// Return `true` if the vector `v` has no elements and `false` otherwise.
    public fun is_empty<T>(self: &BigVector<T>): bool {
        self.length() == 0
    }

    #[test]
    fun big_vector_test() {
        let v = empty(5);
        let i = 0;
        while (i < 100) {
            v.push_back(i);
            i += 1;
        };
        let j = 0;
        while (j < 100) {
            let val = v.borrow(j);
            assert!(*val == j, 0);
            j += 1;
        };
        while (i > 0) {
            i -= 1;
            let (exist, index) = v.index_of(&i);
            let j = v.pop_back();
            assert!(exist, 0);
            assert!(index == i, 0);
            assert!(j == i, 0);
        };
        while (i < 100) {
            v.push_back(i);
            i += 1;
        };
        let last_index = v.length() - 1;
        assert!(v.swap_remove(last_index) == 99, 0);
        assert!(v.swap_remove(0) == 0, 0);
        while (v.length() > 0) {
            // the vector is always [N, 1, 2, ... N-1] with repetitive swap_remove(&mut v, 0)
            let expected = v.length();
            let val = v.swap_remove(0);
            assert!(val == expected, 0);
        };
        v.destroy_empty();
    }

    #[test]
    fun big_vector_append_edge_case_test() {
        let v1 = empty(5);
        let v2 = singleton(1u64, 7);
        let v3 = empty(6);
        let v4 = empty(8);
        v3.append(v4);
        assert!(v3.length() == 0, 0);
        v2.append(v3);
        assert!(v2.length() == 1, 0);
        v1.append(v2);
        assert!(v1.length() == 1, 0);
        v1.destroy();
    }

    #[test]
    fun big_vector_append_test() {
        let v1 = empty(5);
        let v2 = empty(7);
        let i = 0;
        while (i < 7) {
            v1.push_back(i);
            i += 1;
        };
        while (i < 25) {
            v2.push_back(i);
            i += 1;
        };
        v1.append(v2);
        assert!(v1.length() == 25, 0);
        i = 0;
        while (i < 25) {
            assert!(*v1.borrow(i) == i, 0);
            i += 1;
        };
        v1.destroy();
    }

    #[test]
    fun big_vector_to_vector_test() {
        let v1 = empty(7);
        let i = 0;
        while (i < 100) {
            v1.push_back(i);
            i += 1;
        };
        let v2 = v1.to_vector();
        let j = 0;
        while (j < 100) {
            assert!(v2[j] == j, 0);
            j += 1;
        };
        v1.destroy();
    }

    #[test]
    fun big_vector_remove_and_reverse_test() {
        let v = empty(11);
        let i = 0;
        while (i < 101) {
            v.push_back(i);
            i += 1;
        };
        v.remove(100);
        v.remove(90);
        v.remove(80);
        v.remove(70);
        v.remove(60);
        v.remove(50);
        v.remove(40);
        v.remove(30);
        v.remove(20);
        v.remove(10);
        v.remove(0);
        assert!(v.length() == 90, 0);

        let index = 0;
        i = 0;
        while (i < 101) {
            if (i % 10 != 0) {
                assert!(*v.borrow(index) == i, 0);
                index += 1;
            };
            i += 1;
        };
        v.destroy();
    }

    #[test]
    fun big_vector_swap_test() {
        let v = empty(11);
        let i = 0;
        while (i < 101) {
            v.push_back(i);
            i += 1;
        };
        i = 0;
        while (i < 51) {
            v.swap(i, 100 - i);
            i += 1;
        };
        i = 0;
        while (i < 101) {
            assert!(*v.borrow(i) == 100 - i, 0);
            i += 1;
        };
        v.destroy();
    }

    #[test]
    fun big_vector_index_of_test() {
        let v = empty(11);
        let i = 0;
        while (i < 100) {
            v.push_back(i);
            let (found, idx) = v.index_of(&i);
            assert!(found && idx == i, 0);
            i += 1;
        };
        v.destroy();
    }

    #[test]
    fun big_vector_empty_contains() {
        let v = empty<u64>(10);
        assert!(!v.contains::<u64>(&(1 as u64)), 0);
        v.destroy_empty();
    }
}
