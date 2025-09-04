module velor_std::smart_vector {
    use std::error;
    use velor_std::big_vector::{Self, BigVector};
    use velor_std::math64::max;
    use velor_std::type_info::size_of_val;
    use std::option::{Self, Option};

    /// Vector index is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 1;
    /// Cannot destroy a non-empty vector
    const EVECTOR_NOT_EMPTY: u64 = 2;
    /// Cannot pop back from an empty vector
    const EVECTOR_EMPTY: u64 = 3;
    /// bucket_size cannot be 0
    const EZERO_BUCKET_SIZE: u64 = 4;
    /// The length of the smart vectors are not equal.
    const ESMART_VECTORS_LENGTH_MISMATCH: u64 = 0x20005;

    /// A Scalable vector implementation based on tables, Ts are grouped into buckets with `bucket_size`.
    /// The option wrapping BigVector saves space in the metadata associated with BigVector when smart_vector is
    /// so small that inline_vec vector can hold all the data.
    struct SmartVector<T> has store {
        inline_vec: vector<T>,
        big_vec: Option<BigVector<T>>,
        inline_capacity: Option<u64>,
        bucket_size: Option<u64>,
    }

    /// Regular Vector API

    /// Create an empty vector using default logic to estimate `inline_capacity` and `bucket_size`, which may be
    /// inaccurate.
    /// This is exactly the same as empty() but is more standardized as all other data structures have new().
    public fun new<T: store>(): SmartVector<T> {
        empty()
    }

    #[deprecated]
    /// Create an empty vector using default logic to estimate `inline_capacity` and `bucket_size`, which may be
    /// inaccurate.
    public fun empty<T: store>(): SmartVector<T> {
        SmartVector {
            inline_vec: vector[],
            big_vec: option::none(),
            inline_capacity: option::none(),
            bucket_size: option::none(),
        }
    }

    /// Create an empty vector with customized config.
    /// When inline_capacity = 0, SmartVector degrades to a wrapper of BigVector.
    public fun empty_with_config<T: store>(inline_capacity: u64, bucket_size: u64): SmartVector<T> {
        assert!(bucket_size > 0, error::invalid_argument(EZERO_BUCKET_SIZE));
        SmartVector {
            inline_vec: vector[],
            big_vec: option::none(),
            inline_capacity: option::some(inline_capacity),
            bucket_size: option::some(bucket_size),
        }
    }

    /// Create a vector of length 1 containing the passed in T.
    public fun singleton<T: store>(element: T): SmartVector<T> {
        let v = empty();
        v.push_back(element);
        v
    }

    /// Destroy the vector `self`.
    /// Aborts if `self` is not empty.
    public fun destroy_empty<T>(self: SmartVector<T>) {
        assert!(self.is_empty(), error::invalid_argument(EVECTOR_NOT_EMPTY));
        let SmartVector { inline_vec, big_vec, inline_capacity: _, bucket_size: _ } = self;
        inline_vec.destroy_empty();
        big_vec.destroy_none();
    }

    /// Destroy a vector completely when T has `drop`.
    public fun destroy<T: drop>(self: SmartVector<T>) {
        self.clear();
        self.destroy_empty();
    }

    /// Clear a vector completely when T has `drop`.
    public fun clear<T: drop>(self: &mut SmartVector<T>) {
        self.inline_vec = vector[];
        if (self.big_vec.is_some()) {
            self.big_vec.extract().destroy();
        }
    }

    /// Acquire an immutable reference to the `i`th T of the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow<T>(self: &SmartVector<T>, i: u64): &T {
        assert!(i < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let inline_len = self.inline_vec.length();
        if (i < inline_len) {
            self.inline_vec.borrow(i)
        } else {
            self.big_vec.borrow().borrow(i - inline_len)
        }
    }

    /// Return a mutable reference to the `i`th T in the vector `self`.
    /// Aborts if `i` is out of bounds.
    public fun borrow_mut<T>(self: &mut SmartVector<T>, i: u64): &mut T {
        assert!(i < self.length(), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let inline_len = self.inline_vec.length();
        if (i < inline_len) {
            self.inline_vec.borrow_mut(i)
        } else {
            self.big_vec.borrow_mut().borrow_mut(i - inline_len)
        }
    }

    /// Empty and destroy the other vector, and push each of the Ts in the other vector onto the self vector in the
    /// same order as they occurred in other.
    /// Disclaimer: This function may be costly. Use it at your own discretion.
    public fun append<T: store>(self: &mut SmartVector<T>, other: SmartVector<T>) {
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

    /// Add multiple values to the vector at once.
    public fun add_all<T: store>(self: &mut SmartVector<T>, vals: vector<T>) {
        vals.for_each(|val| { self.push_back(val); })
    }

    /// Convert a smart vector to a native vector, which is supposed to be called mostly by view functions to get an
    /// atomic view of the whole vector.
    /// Disclaimer: This function may be costly as the smart vector may be huge in size. Use it at your own discretion.
    public fun to_vector<T: store + copy>(self: &SmartVector<T>): vector<T> {
        let res = self.inline_vec;
        if (self.big_vec.is_some()) {
            let big_vec = self.big_vec.borrow();
            res.append(big_vec.to_vector());
        };
        res
    }

    /// Add T `val` to the end of the vector `self`. It grows the buckets when the current buckets are full.
    /// This operation will cost more gas when it adds new bucket.
    public fun push_back<T: store>(self: &mut SmartVector<T>, val: T) {
        let len = self.length();
        let inline_len = self.inline_vec.length();
        if (len == inline_len) {
            let bucket_size = if (self.inline_capacity.is_some()) {
                if (len < *self.inline_capacity.borrow()) {
                    self.inline_vec.push_back(val);
                    return
                };
                *self.bucket_size.borrow()
            } else {
                let val_size = size_of_val(&val);
                if (val_size * (inline_len + 1) < 150 /* magic number */) {
                    self.inline_vec.push_back(val);
                    return
                };
                let estimated_avg_size = max((size_of_val(&self.inline_vec) + val_size) / (inline_len + 1), 1);
                max(1024 /* free_write_quota */ / estimated_avg_size, 1)
            };
            self.big_vec.fill(big_vector::empty(bucket_size));
        };
        self.big_vec.borrow_mut().push_back(val);
    }

    /// Pop an T from the end of vector `self`. It does shrink the buckets if they're empty.
    /// Aborts if `self` is empty.
    public fun pop_back<T>(self: &mut SmartVector<T>): T {
        assert!(!self.is_empty(), error::invalid_state(EVECTOR_EMPTY));
        let big_vec_wrapper = &mut self.big_vec;
        if (big_vec_wrapper.is_some()) {
            let big_vec = big_vec_wrapper.extract();
            let val = big_vec.pop_back();
            if (big_vec.is_empty()) {
                big_vec.destroy_empty()
            } else {
                big_vec_wrapper.fill(big_vec);
            };
            val
        } else {
            self.inline_vec.pop_back()
        }
    }

    /// Remove the T at index i in the vector self and return the owned value that was previously stored at i in self.
    /// All Ts occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
    /// Disclaimer: This function may be costly. Use it at your own discretion.
    public fun remove<T>(self: &mut SmartVector<T>, i: u64): T {
        let len = self.length();
        assert!(i < len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let inline_len = self.inline_vec.length();
        if (i < inline_len) {
            self.inline_vec.remove(i)
        } else {
            let big_vec_wrapper = &mut self.big_vec;
            let big_vec = big_vec_wrapper.extract();
            let val = big_vec.remove(i - inline_len);
            if (big_vec.is_empty()) {
                big_vec.destroy_empty()
            } else {
                big_vec_wrapper.fill(big_vec);
            };
            val
        }
    }

    /// Swap the `i`th T of the vector `self` with the last T and then pop the vector.
    /// This is O(1), but does not preserve ordering of Ts in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<T>(self: &mut SmartVector<T>, i: u64): T {
        let len = self.length();
        assert!(i < len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let inline_len = self.inline_vec.length();
        let big_vec_wrapper = &mut self.big_vec;
        let inline_vec = &mut self.inline_vec;
        if (i >= inline_len) {
            let big_vec = big_vec_wrapper.extract();
            let val = big_vec.swap_remove(i - inline_len);
            if (big_vec.is_empty()) {
                big_vec.destroy_empty()
            } else {
                big_vec_wrapper.fill(big_vec);
            };
            val
        } else {
            if (inline_len < len) {
                let big_vec = big_vec_wrapper.extract();
                let last_from_big_vec = big_vec.pop_back();
                if (big_vec.is_empty()) {
                    big_vec.destroy_empty()
                } else {
                    big_vec_wrapper.fill(big_vec);
                };
                inline_vec.push_back(last_from_big_vec);
            };
            inline_vec.swap_remove(i)
        }
    }

    /// Swap the Ts at the i'th and j'th indices in the vector v. Will abort if either of i or j are out of bounds
    /// for self.
    public fun swap<T: store>(self: &mut SmartVector<T>, i: u64, j: u64) {
        if (i > j) {
            return self.swap(j, i)
        };
        let len = self.length();
        assert!(j < len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
        let inline_len = self.inline_vec.length();
        if (i >= inline_len) {
            self.big_vec.borrow_mut().swap(i - inline_len, j - inline_len);
        } else if (j < inline_len) {
            self.inline_vec.swap(i, j);
        } else {
            let big_vec = self.big_vec.borrow_mut();
            let inline_vec = &mut self.inline_vec;
            let element_i = inline_vec.swap_remove(i);
            let element_j = big_vec.swap_remove(j - inline_len);
            inline_vec.push_back(element_j);
            inline_vec.swap(i, inline_len - 1);
            big_vec.push_back(element_i);
            big_vec.swap(j - inline_len, len - inline_len - 1);
        }
    }

    /// Reverse the order of the Ts in the vector self in-place.
    /// Disclaimer: This function may be costly. Use it at your own discretion.
    public fun reverse<T: store>(self: &mut SmartVector<T>) {
        let inline_len = self.inline_vec.length();
        let new_inline_vec = vector[];
        // Push the last `inline_len` Ts into a temp vector.
        for (i in 0..inline_len) {
            new_inline_vec.push_back(self.pop_back());
        };
        new_inline_vec.reverse();
        // Reverse the big_vector left if exists.
        if (self.big_vec.is_some()) {
            self.big_vec.borrow_mut().reverse();
        };
        // Mem::swap the two vectors.
        let temp_vec = vector[];
        while (!self.inline_vec.is_empty()) {
            temp_vec.push_back(self.inline_vec.pop_back());
        };
        temp_vec.reverse();
        while (!new_inline_vec.is_empty()) {
            self.inline_vec.push_back(new_inline_vec.pop_back());
        };
        new_inline_vec.destroy_empty();
        // Push the rest Ts originally left in inline_vector back to the end of the smart vector.
        while (!temp_vec.is_empty()) {
            self.push_back(temp_vec.pop_back());
        };
        temp_vec.destroy_empty();
    }

    /// Return `(true, i)` if `val` is in the vector `self` at index `i`.
    /// Otherwise, returns `(false, 0)`.
    /// Disclaimer: This function may be costly. Use it at your own discretion.
    public fun index_of<T>(self: &SmartVector<T>, val: &T): (bool, u64) {
        let (found, i) = self.inline_vec.index_of(val);
        if (found) {
            (true, i)
        } else if (self.big_vec.is_some()) {
            let (found, i) = self.big_vec.borrow().index_of(val);
            (found, i + self.inline_vec.length())
        } else {
            (false, 0)
        }
    }

    /// Return true if `val` is in the vector `self`.
    /// Disclaimer: This function may be costly. Use it at your own discretion.
    public fun contains<T>(self: &SmartVector<T>, val: &T): bool {
        if (self.is_empty()) return false;
        let (exist, _) = self.index_of(val);
        exist
    }

    /// Return the length of the vector.
    public fun length<T>(self: &SmartVector<T>): u64 {
        self.inline_vec.length() + if (self.big_vec.is_none()) {
            0
        } else {
            self.big_vec.borrow().length()
        }
    }

    /// Return `true` if the vector `self` has no Ts and `false` otherwise.
    public fun is_empty<T>(self: &SmartVector<T>): bool {
        self.length() == 0
    }

    /// Apply the function to each T in the vector, consuming it.
    public inline fun for_each<T: store>(self: SmartVector<T>, f: |T|) {
        self.reverse(); // We need to reverse the vector to consume it efficiently
        self.for_each_reverse(|e| f(e));
    }

    /// Apply the function to each T in the vector, consuming it.
    public inline fun for_each_reverse<T>(self: SmartVector<T>, f: |T|) {
        let len = self.length();
        while (len > 0) {
            f(self.pop_back());
            len -= 1;
        };
        self.destroy_empty()
    }

    /// Apply the function to a reference of each T in the vector.
    public inline fun for_each_ref<T>(self: &SmartVector<T>, f: |&T|) {
        let len = self.length();
        for (i in 0..len) {
            f(self.borrow(i));
        }
    }

    /// Apply the function to a mutable reference to each T in the vector.
    public inline fun for_each_mut<T>(self: &mut SmartVector<T>, f: |&mut T|) {
        let len = self.length();
        for (i in 0..len) {
            f(self.borrow_mut(i));
        }
    }

    /// Apply the function to a reference of each T in the vector with its index.
    public inline fun enumerate_ref<T>(self: &SmartVector<T>, f: |u64, &T|) {
        let len = self.length();
        for (i in 0..len) {
            f(i, self.borrow(i));
        };
    }

    /// Apply the function to a mutable reference of each T in the vector with its index.
    public inline fun enumerate_mut<T>(self: &mut SmartVector<T>, f: |u64, &mut T|) {
        let len = self.length();
        for (i in 0..len) {
            f(i, self.borrow_mut(i));
        };
    }

    /// Fold the function over the Ts. For example, `fold(vector[1,2,3], 0, f)` will execute
    /// `f(f(f(0, 1), 2), 3)`
    public inline fun fold<Accumulator, T: store>(
        self: SmartVector<T>,
        init: Accumulator,
        f: |Accumulator, T|Accumulator
    ): Accumulator {
        let accu = init;
        self.for_each(|elem| accu = f(accu, elem));
        accu
    }

    /// Fold right like fold above but working right to left. For example, `fold(vector[1,2,3], 0, f)` will execute
    /// `f(1, f(2, f(3, 0)))`
    public inline fun foldr<Accumulator, T>(
        self: SmartVector<T>,
        init: Accumulator,
        f: |T, Accumulator|Accumulator
    ): Accumulator {
        let accu = init;
        self.for_each_reverse(|elem| accu = f(elem, accu));
        accu
    }

    /// Map the function over the references of the Ts of the vector, producing a new vector without modifying the
    /// original vector.
    public inline fun map_ref<T1, T2: store>(
        self: &SmartVector<T1>,
        f: |&T1|T2
    ): SmartVector<T2> {
        let result = velor_std::smart_vector::new<T2>();
        self.for_each_ref(|elem| result.push_back(f(elem)));
        result
    }

    /// Map the function over the Ts of the vector, producing a new vector.
    public inline fun map<T1: store, T2: store>(
        self: SmartVector<T1>,
        f: |T1|T2
    ): SmartVector<T2> {
        let result = velor_std::smart_vector::new<T2>();
        self.for_each(|elem| result.push_back(f(elem)));
        result
    }

    /// Filter the vector using the boolean function, removing all Ts for which `p(e)` is not true.
    public inline fun filter<T: store + drop>(
        self: SmartVector<T>,
        p: |&T|bool
    ): SmartVector<T> {
        let result = velor_std::smart_vector::new<T>();
        self.for_each(|elem| {
            if (p(&elem)) result.push_back(elem);
        });
        result
    }

    public inline fun zip<T1: store, T2: store>(self: SmartVector<T1>, v2: SmartVector<T2>, f: |T1, T2|) {
        // We need to reverse the vectors to consume it efficiently
        self.reverse();
        v2.reverse();
        self.zip_reverse(v2, |e1, e2| f(e1, e2));
    }

    /// Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_reverse<T1, T2>(
        self: SmartVector<T1>,
        v2: SmartVector<T2>,
        f: |T1, T2|,
    ) {
        let len = self.length();
        // We can't use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20005);
        while (len > 0) {
            f(self.pop_back(), v2.pop_back());
            len -= 1;
        };
        self.destroy_empty();
        v2.destroy_empty();
    }

    /// Apply the function to the references of each pair of elements in the two given vectors.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_ref<T1, T2>(
        self: &SmartVector<T1>,
        v2: &SmartVector<T2>,
        f: |&T1, &T2|,
    ) {
        let len = self.length();
        // We can't use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20005);
        for (i in 0..len) {
            f(self.borrow(i), v2.borrow(i));
        }
    }

    /// Apply the function to mutable references to each pair of elements in the two given vectors.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_mut<T1, T2>(
        self: &mut SmartVector<T1>,
        v2: &mut SmartVector<T2>,
        f: |&mut T1, &mut T2|,
    ) {
        let len = self.length();
        // We can't use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20005);
        for (i in 0..len) {
            f(self.borrow_mut(i), v2.borrow_mut(i));
        }
    }

    /// Map the function over the element pairs of the two vectors, producing a new vector.
    public inline fun zip_map<T1: store, T2: store, NewT: store>(
        self: SmartVector<T1>,
        v2: SmartVector<T2>,
        f: |T1, T2|NewT
    ): SmartVector<NewT> {
        // We can't use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(self.length() == v2.length(), 0x20005);

        let result = velor_std::smart_vector::new<NewT>();
        self.zip(v2, |e1, e2| result.push_back(f(e1, e2)));
        result
    }

    /// Map the function over the references of the element pairs of two vectors, producing a new vector from the return
    /// values without modifying the original vectors.
    public inline fun zip_map_ref<T1, T2, NewT: store>(
        self: &SmartVector<T1>,
        v2: &SmartVector<T2>,
        f: |&T1, &T2|NewT
    ): SmartVector<NewT> {
        // We can't use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(self.length() == v2.length(), 0x20005);

        let result = velor_std::smart_vector::new<NewT>();
        self.zip_ref(v2, |e1, e2| result.push_back(f(e1, e2)));
        result
    }

    #[test]
    fun smart_vector_test() {
        let v = empty();
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
    fun smart_vector_append_edge_case_test() {
        let v1 = empty();
        let v2 = singleton(1u64);
        let v3 = empty();
        let v4 = empty();
        v3.append(v4);
        assert!(v3.length() == 0, 0);
        v2.append(v3);
        assert!(v2.length() == 1, 0);
        v1.append(v2);
        assert!(v1.length() == 1, 0);
        v1.destroy();
    }

    #[test]
    fun smart_vector_append_test() {
        let v1 = empty();
        let v2 = empty();
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
    fun smart_vector_remove_test() {
        let v = empty();
        let i = 0u64;
        while (i < 101) {
            v.push_back(i);
            i += 1;
        };
        let inline_len = v.inline_vec.length();
        v.remove(100);
        v.remove(90);
        v.remove(80);
        v.remove(70);
        v.remove(60);
        v.remove(50);
        v.remove(40);
        v.remove(30);
        v.remove(20);
        assert!(v.inline_vec.length() == inline_len, 0);
        v.remove(10);
        assert!(v.inline_vec.length() + 1 == inline_len, 0);
        v.remove(0);
        assert!(v.inline_vec.length() + 2 == inline_len, 0);
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
    fun smart_vector_reverse_test() {
        let v = empty();
        let i = 0u64;
        while (i < 10) {
            v.push_back(i);
            i += 1;
        };
        v.reverse();
        let k = 0;
        while (k < 10) {
            assert!(v.inline_vec[k] == 9 - k, 0);
            k += 1;
        };
        while (i < 100) {
            v.push_back(i);
            i += 1;
        };
        while (!v.inline_vec.is_empty()) {
            v.remove(0);
        };
        v.reverse();
        i = 0;
        let len = v.length();
        while (i + 1 < len) {
            assert!(
                *v.big_vec.borrow().borrow(i) == *v.big_vec.borrow().borrow(i + 1) + 1,
                0
            );
            i += 1;
        };
        v.destroy();
    }

    #[test]
    fun smart_vector_add_all_test() {
        let v = empty_with_config(1, 2);
        v.add_all(vector[1, 2, 3, 4, 5, 6]);
        assert!(v.length() == 6, 0);
        let i = 0;
        while (i < 6) {
            assert!(*v.borrow(i) == i + 1, 0);
            i += 1;
        };
        v.destroy();
    }

    #[test]
    fun smart_vector_to_vector_test() {
        let v1 = empty_with_config(7, 11);
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
    fun smart_vector_swap_test() {
        let v = empty();
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
    fun smart_vector_index_of_test() {
        let v = empty();
        let i = 0;
        while (i < 100) {
            v.push_back(i);
            let (found, idx) = v.index_of(&i);
            assert!(found && idx == i, 0);
            i += 1;
        };
        v.destroy();
    }
}
