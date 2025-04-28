#[test_only]
module std::vector_tests {
    use std::vector as V;
    use std::vector;

    struct R has store {}

    struct Droppable has drop {}

    struct NotDroppable {}

    #[test]
    fun test_singleton_contains() {
        assert!(V::singleton(0)[0] == 0, 0);
        assert!(V::singleton(true)[0] == true, 0);
        assert!(V::singleton(@0x1)[0] == @0x1, 0);
    }

    #[test]
    fun test_singleton_len() {
        assert!(V::singleton(0).length() == 1, 0);
        assert!(V::singleton(true).length() == 1, 0);
        assert!(V::singleton(@0x1).length() == 1, 0);
    }

    #[test]
    fun test_empty_is_empty() {
        assert!(V::empty<u64>().is_empty(), 0);
    }

    #[test]
    fun append_empties_is_empty() {
        let v1 = V::empty<u64>();
        let v2 = V::empty<u64>();
        v1.append(v2);
        assert!(v1.is_empty(), 0);
    }

    #[test]
    fun append_respects_order_empty_lhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        for (i in 0..4) {
            v2.push_back(i)
        };
        v1.append(v2);
        assert!(!v1.is_empty(), 0);
        assert!(v1.length() == 4, 1);
        assert!(v1[0] == 0, 2);
        assert!(v1[1] == 1, 3);
        assert!(v1[2] == 2, 4);
        assert!(v1[3] == 3, 5);
    }

    #[test]
    fun append_respects_order_empty_rhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        for (i in 0..4) {
            v1.push_back(i)
        };
        v1.append(v2);
        assert!(!v1.is_empty(), 0);
        assert!(v1.length() == 4, 1);
        assert!(v1[0] == 0, 2);
        assert!(v1[1] == 1, 3);
        assert!(v1[2] == 2, 4);
        assert!(v1[3] == 3, 5);
    }

    #[test]
    fun append_respects_order_nonempty_rhs_lhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        for (i in 0..4) {
            v1.push_back(i)
        };
        for (i in 4..8) {
            v2.push_back(i)
        };
        v1.append(v2);
        assert!(!v1.is_empty(), 0);
        assert!(v1.length() == 8, 1);
        for (i in 0..8) {
            assert!(v1[i] == i, i);
        }
    }

    #[test]
    fun test_trim() {
        {
            let v = V::empty<u64>();
            assert!(&v.trim(0) == &vector[], 0);
        };
        {
            let v = vector[1];
            assert!(&v.trim(1) == &vector[], 1);
            assert!(&v.trim(0) == &vector[1], 2);
        };
        {
            let v = vector[1, 2];
            assert!(&v.trim(0) == &vector[1, 2], 3);
        };
        {
            let v = vector[1, 2, 3, 4, 5, 6];
            let other = v.trim(4);
            assert!(v == vector[1, 2, 3, 4], 4);
            assert!(other == vector[5, 6], 5);

            let other_empty = v.trim(4);
            assert!(v == vector[1, 2, 3, 4], 6);
            assert!(other_empty == vector[], 7);
        };
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun test_trim_fail() {
        let v = vector[1];
        v.trim(2);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun test_trim_fail_2() {
        let v = vector[1, 2, 3];
        v.trim(4);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 1, location = Self)]
    fun borrow_out_of_range() {
        let v = vector[7];
        v.borrow(1);
    }

    #[test]
    fun vector_contains() {
        let vec = V::empty();
        assert!(!vec.contains(&0), 1);

        vec.push_back(0);
        assert!(vec.contains(&0), 2);
        assert!(!vec.contains(&1), 3);

        vec.push_back(1);
        assert!(vec.contains(&0), 4);
        assert!(vec.contains(&1), 5);
        assert!(!vec.contains(&2), 6);

        vec.push_back(2);
        assert!(vec.contains(&0), 7);
        assert!(vec.contains(&1), 8);
        assert!(vec.contains(&2), 9);
        assert!(!vec.contains(&3), 10);
    }

    #[test]
    fun destroy_empty() {
        V::empty<u64>().destroy_empty();
        V::empty<R>().destroy_empty();
    }

    #[test]
    fun destroy_empty_with_pops() {
        let v = vector[42];
        v.pop_back();
        v.destroy_empty();
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 3, location = Self)]
    fun destroy_non_empty() {
        let v = vector[42];
        v.destroy_empty();
    }

    #[test]
    fun get_set_work() {
        let vec = vector[0, 1];
        assert!(vec[1] == 1, 0);
        assert!(vec[0] == 0, 1);

        vec[0] = 17;
        assert!(vec[1] == 1, 0);
        assert!(vec[0] == 17, 0);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 2, location = Self)]
    fun pop_out_of_range() {
        let v = V::empty<u64>();
        v.pop_back();
    }

    #[test]
    fun swap_different_indices() {
        let vec = vector[0, 1, 2, 3];
        vec.swap(0, 3);
        vec.swap(1, 2);
        assert!(vec[0] == 3);
        assert!(vec[1] == 2);
        assert!(vec[2] == 1);
        assert!(vec[3] == 0);
    }

    #[test]
    fun swap_same_index() {
        let vec = vector[0, 1, 2, 3];
        vec.swap(1, 1);
        assert!(vec[0] == 0, 0);
        assert!(vec[1] == 1, 0);
        assert!(vec[2] == 2, 0);
        assert!(vec[3] == 3, 0);
    }

    #[test]
    fun remove_singleton_vector() {
        let v = V::singleton(0);
        assert!(v.remove(0) == 0, 0);
        assert!(v.length() == 0, 0);
    }

    #[test]
    fun remove_nonsingleton_vector() {
        let v = vector[0, 1, 2, 3];

        assert!(v.remove(1) == 1, 0);
        assert!(v.length() == 3, 0);
        assert!(v[0] == 0, 0);
        assert!(v[1] == 2, 0);
        assert!(v[2] == 3, 0);
    }

    #[test]
    fun remove_nonsingleton_vector_last_elem() {
        let v = vector[0, 1, 2, 3];

        assert!(v.remove(3) == 3, 0);
        assert!(v.length() == 3, 0);
        assert!(v[0] == 0, 0);
        assert!(v[1] == 1, 0);
        assert!(v[2] == 2, 0);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun remove_empty_vector() {
        let v = V::empty<u64>();
        v.remove(0);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun remove_out_of_bound_index() {
        let v = vector[0];
        v.remove(1);
    }

    fun remove_more_cases() {
        let v: vector<u64> = vector[1];
        assert!(v.remove(0) == 1, 1);
        assert!(&v == &vector[], 1);

        let v: vector<u64> = vector[2, 1];
        assert!(v.remove(0) == 2, 1);
        assert!(&v == &vector[1], 1);

        let v: vector<u64> = vector[1, 2];
        assert!(v.remove(1) == 2, 1);
        assert!(&v == &vector[1], 1);

        let v: vector<u64> = vector[3, 1, 2];
        assert!(v.remove(0) == 3, 1);
        assert!(&v == &vector[1, 2], 1);

        let v: vector<u64> = vector[1, 3, 2];
        assert!(v.remove(1) == 3, 1);
        assert!(&v == &vector[1, 2], 1);

        let v: vector<u64> = vector[1, 2, 3];
        assert!(v.remove(2) == 3, 1);
        assert!(&v == &vector[1, 2], 1);

        let v: vector<u64> = vector[4, 1, 2, 3];
        assert!(v.remove(0) == 4, 1);
        assert!(&v == &vector[1, 2, 3], 1);

        let v: vector<u64> = vector[5, 1, 2, 3, 4];
        assert!(v.remove(0) == 5, 1);
        assert!(&v == &vector[1, 2, 3, 4], 1);

        let v: vector<u64> = vector[1, 5, 2, 3, 4];
        assert!(v.remove(1) == 5, 1);
        assert!(&v == &vector[1, 2, 3, 4], 1);

        let v: vector<u64> = vector[1, 2, 5, 3, 4];
        assert!(v.remove(2) == 5, 1);
        assert!(&v == &vector[1, 2, 3, 4], 1);

        let v: vector<u64> = vector[1, 2, 3, 4, 5];
        assert!(v.remove(4) == 5, 1);
        assert!(&v == &vector[1, 2, 3, 4], 1);
    }

    #[test]
    fun remove_value_singleton_vector() {
        let v = vector[0];
        assert!(v.remove_value(&0)[0] == 0, 0);
        assert!(v.length() == 0, 0);
    }

    #[test]
    fun remove_value_nonsingleton_vector() {
        let v = vector[0, 1, 2, 3];

        assert!(v.remove_value(&2)[0] == 2, 0);
        assert!(v.length() == 3, 0);
        assert!(v[0] == 0, 0);
        assert!(v[1] == 1, 0);
        assert!(v[2] == 3, 0);
    }

    #[test]
    fun remove_value_nonsingleton_vector_last_elem() {
        let v = vector[0, 1, 2, 3];

        assert!(v.remove_value(&3)[0] == 3, 0);
        assert!(v.length() == 3, 0);
        assert!(v[0] == 0, 0);
        assert!(v[1] == 1, 0);
        assert!(v[2] == 2, 0);
    }

    #[test]
    fun remove_value_empty_vector() {
        let v = V::empty<u64>();
        assert!(v.remove_value(&1).length() == 0, 0);
        assert!(v.length() == 0, 1);
    }

    #[test]
    fun remove_value_nonexistent() {
        let v = vector[0];
        assert!(v.remove_value(&1).length() == 0, 0);
        assert!(v.length() == 1, 1);
    }

    #[test]
    fun reverse_vector_empty() {
        let v = V::empty<u64>();
        let is_empty = v.is_empty();
        v.reverse();
        assert!(is_empty == v.is_empty(), 0);
    }

    #[test]
    fun reverse_singleton_vector() {
        let v = V::singleton(0);
        assert!(v[0] == 0, 1);
        v.reverse();
        assert!(v[0] == 0, 2);
    }

    #[test]
    fun reverse_vector_nonempty_even_length() {
        let v = vector[0, 1, 2, 3];

        assert!(v[0] == 0, 1);
        assert!(v[1] == 1, 2);
        assert!(v[2] == 2, 3);
        assert!(v[3] == 3, 4);

        v.reverse();

        assert!(v[3] == 0, 5);
        assert!(v[2] == 1, 6);
        assert!(v[1] == 2, 7);
        assert!(v[0] == 3, 8);
    }

    #[test]
    fun reverse_vector_nonempty_odd_length_non_singleton() {
        let v = vector[0, 1, 2];

        assert!(v[0] == 0, 1);
        assert!(v[1] == 1, 2);
        assert!(v[2] == 2, 3);

        v.reverse();

        assert!(v[2] == 0, 4);
        assert!(v[1] == 1, 5);
        assert!(v[0] == 2, 6);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 1, location = Self)]
    fun swap_empty() {
        let v = V::empty<u64>();
        v.swap(0, 0);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 1, location = Self)]
    fun swap_out_of_range() {
        let v = vector[0, 1, 2, 3];

        v.swap(1, 10);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun swap_remove_empty() {
        let v = V::empty<u64>();
        v.swap_remove(0);
    }

    #[test]
    fun swap_remove_singleton() {
        let v = vector[0];
        assert!(v.swap_remove(0) == 0, 0);
        assert!(v.is_empty(), 1);
    }

    #[test]
    fun swap_remove_inside_vector() {
        let v = vector[0, 1, 2, 3];

        assert!(v[0] == 0, 1);
        assert!(v[1] == 1, 2);
        assert!(v[2] == 2, 3);
        assert!(v[3] == 3, 4);

        assert!(v.swap_remove(1) == 1, 5);
        assert!(v.length() == 3, 6);

        assert!(v[0] == 0, 7);
        assert!(v[1] == 3, 8);
        assert!(v[2] == 2, 9);
    }

    #[test]
    fun swap_remove_end_of_vector() {
        let v = vector[0, 1, 2, 3];

        assert!(v[0] == 0, 1);
        assert!(v[1] == 1, 2);
        assert!(v[2] == 2, 3);
        assert!(v[3] == 3, 4);

        assert!(v.swap_remove(3) == 3, 5);
        assert!(v.length() == 3, 6);

        assert!(v[0] == 0, 7);
        assert!(v[1] == 1, 8);
        assert!(v[2] == 2, 9);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 1, location = std::vector)]
    fun swap_remove_out_of_range() {
        let v = vector[0];
        v.swap_remove(1);
    }

    #[test]
    fun push_back_and_borrow() {
        let v = V::empty();
        v.push_back(7);
        assert!(!v.is_empty(), 0);
        assert!(v.length() == 1, 1);
        assert!(v[0] == 7, 2);

        v.push_back(8);
        assert!(v.length() == 2, 3);
        assert!(v[0] == 7, 4);
        assert!(v[1] == 8, 5);
    }

    #[test]
    fun index_of_empty_not_has() {
        let v = V::empty();
        let (has, index) = v.index_of(&true);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    #[test]
    fun index_of_nonempty_not_has() {
        let v = vector[false];
        let (has, index) = v.index_of(&true);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    #[test]
    fun index_of_nonempty_has() {
        let v = vector[false, true];
        let (has, index) = v.index_of(&true);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    // index_of will return the index first occurence that is equal
    #[test]
    fun index_of_nonempty_has_multiple_occurences() {
        let v = vector[false, true, true];
        let (has, index) = v.index_of(&true);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    #[test]
    fun find_empty_not_has() {
        let v = V::empty<u64>();
        let (has, index) = v.find(|_x| true);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    #[test]
    fun find_nonempty_not_has() {
        let v = vector[1, 2];
        let (has, index) = v.find(|x| *x == 3);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    #[test]
    fun find_nonempty_has() {
        let v = vector[1, 2, 3];
        let (has, index) = v.find(|x| *x == 2);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    #[test]
    fun find_nonempty_has_multiple_occurences() {
        let v = vector[1, 2, 2, 3];
        let (has, index) = v.find(|x| *x == 2);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    #[test]
    fun length() {
        let empty = V::empty();
        assert!(empty.length() == 0);
        for (i in 0..42) {
            empty.push_back(i);
            assert!(empty.length() == i + 1, i);
        }
    }

    #[test]
    fun pop_push_back() {
        let v = V::empty();
        let i = 0;
        let max_len = 42;

        while (i < max_len) {
            v.push_back(i);
            i += 1;
        };

        while (i > 0) {
            assert!(v.pop_back() == i - 1, i);
            i -= 1;
        };
    }

    #[test_only]
    fun test_natives_with_type<T>(x1: T, x2: T): (T, T) {
        let v = V::empty();
        assert!(v.length() == 0, 0);
        v.push_back(x1);
        assert!(v.length() == 1, 1);
        v.push_back(x2);
        assert!(v.length() == 2, 2);
        v.swap(0, 1);
        x1 = v.pop_back();
        assert!(v.length() == 1, 3);
        x2 = v.pop_back();
        assert!(v.length() == 0, 4);
        v.destroy_empty();
        (x1, x2)
    }

    #[test]
    fun test_natives_with_different_instantiations() {
        test_natives_with_type<u8>(1u8, 2u8);
        test_natives_with_type<u64>(1u64, 2u64);
        test_natives_with_type<u128>(1u128, 2u128);
        test_natives_with_type<bool>(true, false);
        test_natives_with_type<address>(@0x1, @0x2);

        test_natives_with_type<vector<u8>>(V::empty(), V::empty());

        test_natives_with_type<Droppable>(Droppable {}, Droppable {});
        (NotDroppable {}, NotDroppable {}) = test_natives_with_type<NotDroppable>(
            NotDroppable {},
            NotDroppable {}
        );
    }

    #[test]
    fun test_for_each() {
        let v = vector[1, 2, 3];
        let s = 0;
        v.for_each(|e| {
            s += e;
        });
        assert!(s == 6)
    }

    #[test]
    fun test_zip() {
        let v1 = vector[1, 2, 3];
        let v2 = vector[10, 20, 30];
        let s = 0;
        v1.zip(v2, |e1, e2| s += e1 * e2);
        assert!(s == 140, 0);
    }

    #[test]
    // zip is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::EVECTORS_LENGTH_MISMATCH, location = Self)]
    fun test_zip_mismatching_lengths_should_fail() {
        let v1 = vector[1];
        let v2 = vector[10, 20];
        let s = 0;
        v1.zip(v2, |e1, e2| s = s + e1 * e2);
    }

    #[test]
    fun test_enumerate_ref() {
        let v = vector[1, 2, 3];
        let i_s = 0;
        let s = 0;
        v.enumerate_ref(|i, e| {
            i_s += i;
            s += *e;
        });
        assert!(i_s == 3, 0);
        assert!(s == 6, 0);
    }

    #[test]
    fun test_for_each_ref() {
        let v = vector[1, 2, 3];
        let s = 0;
        v.for_each_ref(|e| s += *e);
        assert!(s == 6, 0)
    }

    #[test]
    fun test_for_each_mut() {
        let v = vector[1, 2, 3];
        let s = 2;
        v.for_each_mut(|e| {
            *e = s;
            s += 1
        });
        assert!(v == vector[2, 3, 4], 0)
    }

    #[test]
    fun test_zip_ref() {
        let v1 = vector[1, 2, 3];
        let v2 = vector[10, 20, 30];
        let s = 0;
        v1.zip_ref(&v2, |e1, e2| s += *e1 * *e2);
        assert!(s == 140, 0);
    }

    #[test]
    // zip_ref is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::EVECTORS_LENGTH_MISMATCH, location = Self)]
    fun test_zip_ref_mismatching_lengths_should_fail() {
        let v1 = vector[1];
        let v2 = vector[10, 20];
        let s = 0;
        v1.zip_ref(&v2, |e1, e2| s += *e1 * *e2);
    }

    #[test]
    fun test_zip_mut() {
        let v1 = vector[1, 2, 3];
        let v2 = vector[10, 20, 30];
        v1.zip_mut(&mut v2, |e1, e2| {
            let e1: &mut u64 = e1;
            let e2: &mut u64 = e2;
            *e1 += 1;
            *e2 += 10;
        });
        assert!(v1 == vector[2, 3, 4], 0);
        assert!(v2 == vector[20, 30, 40], 0);
    }

    #[test]
    fun test_zip_map() {
        let v1 = vector[1, 2, 3];
        let v2 = vector[10, 20, 30];
        let result = v1.zip_map(v2, |e1, e2| e1 + e2);
        assert!(result == vector[11, 22, 33], 0);
    }

    #[test]
    fun test_zip_map_ref() {
        let v1 = vector[1, 2, 3];
        let v2 = vector[10, 20, 30];
        let result = v1.zip_map_ref(&v2, |e1, e2| *e1 + *e2);
        assert!(result == vector[11, 22, 33], 0);
    }

    #[test]
    // zip_mut is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::EVECTORS_LENGTH_MISMATCH, location = Self)]
    fun test_zip_mut_mismatching_lengths_should_fail() {
        let v1 = vector[1];
        let v2 = vector[10, 20];
        let s = 0;
        v1.zip_mut(&mut v2, |e1, e2| s = s + *e1 * *e2);
    }

    #[test]
    // zip_map is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::EVECTORS_LENGTH_MISMATCH, location = Self)]
    fun test_zip_map_mismatching_lengths_should_fail() {
        let v1 = vector[1];
        let v2 = vector[10, 20];
        v1.zip_map(v2, |e1, e2| e1 * e2);
    }

    #[test]
    // zip_map_ref is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::EVECTORS_LENGTH_MISMATCH, location = Self)]
    fun test_zip_map_ref_mismatching_lengths_should_fail() {
        let v1 = vector[1];
        let v2 = vector[10, 20];
        v1.zip_map_ref(&v2, |e1, e2| *e1 * *e2);
    }

    #[test]
    fun test_enumerate_mut() {
        let v = vector[1, 2, 3];
        let i_s = 0;
        let s = 2;
        v.enumerate_mut(|i, e| {
            i_s += i;
            *e = s;
            s += 1
        });
        assert!(i_s == 3, 0);
        assert!(v == vector[2, 3, 4], 0);
    }

    #[test]
    fun test_fold() {
        let v = vector[1, 2, 3];
        let s = v.fold(0, |r, e| r + e);
        assert!(s == 6, 0)
    }

    #[test]
    fun test_foldr() {
        // use non-commutative minus operation to test the difference between fold and foldr
        {
            let v = vector[3, 2, 1];
            // ((100 - 3) - 2) - 1 = 94
            let s = v.fold(100, |l, r| l - r);
            assert!(s == 94, 0)
        };
        {
            let v = vector[3, 2, 1];
            // 3 - (2 - (1 - 0)) = 2
            let s = v.foldr(0, |l, r| l - r);
            assert!(s == 2, 1)
        }
    }

    #[test]
    fun test_map() {
        let v = vector[1, 2, 3];
        let s = v.map(|x| x + 1);
        assert!(s == vector[2, 3, 4], 0)
    }

    #[test]
    fun test_map_ref() {
        let v = vector[1, 2, 3];
        let s = v.map_ref(|x| *x + 1);
        assert!(s == vector[2, 3, 4], 0)
    }

    #[test]
    fun test_filter() {
        let v = vector[1, 2, 3];
        let s = v.filter(|x| *x % 2 == 0);
        assert!(s == vector[2], 0)
    }

    #[test]
    fun test_any() {
        let v = vector[1, 2, 3];
        let r = v.any(|x| *x > 2);
        assert!(r, 0)
    }

    #[test]
    fun test_all() {
        let v = vector[1, 2, 3];
        let r = v.all(|x| *x >= 1);
        assert!(r, 0)
    }

    #[test]
    fun test_rotate() {
        let v = vector[1, 2, 3, 4, 5];
        assert!(v.rotate(2) == 3, 0);
        assert!(&v == &vector[3, 4, 5, 1, 2], 1);

        assert!(v.rotate_slice(1, 2, 5) == 4, 2);
        assert!(&v == &vector[3, 5, 1, 2, 4], 3);

        assert!(v.rotate_slice(0, 0, 5) == 5, 2);
        assert!(&v == &vector[3, 5, 1, 2, 4], 3);
        assert!(v.rotate_slice(0, 5, 5) == 0, 2);
        assert!(&v == &vector[3, 5, 1, 2, 4], 3);
    }

    #[test]
    fun test_partition() {
        let v = vector[1, 2, 3, 4, 5];
        assert!(v.partition(|n| *n % 2 == 0) == 2, 0);
        assert!(&v == &vector[2, 4, 3, 1, 5], 1);

        assert!(v.partition(|_n| false) == 0, 0);
        assert!(&v == &vector[2, 4, 3, 1, 5], 1);

        assert!(v.partition(|_n| true) == 5, 0);
        assert!(&v == &vector[2, 4, 3, 1, 5], 1);
    }

    #[test]
    fun test_stable_partition() {
        let v: vector<u64> = vector[1, 2, 3, 4, 5];

        assert!(v.stable_partition(|n| *n % 2 == 0) == 2, 0);
        assert!(&v == &vector[2, 4, 1, 3, 5], 1);

        assert!(v.partition(|_n| false) == 0, 0);
        assert!(&v == &vector[2, 4, 1, 3, 5], 1);

        assert!(v.partition(|_n| true) == 5, 0);
        assert!(&v == &vector[2, 4, 1, 3, 5], 1);
    }

    #[test]
    fun test_insert() {
        let v: vector<u64> = vector[1, 2, 3, 4, 5];

        v.insert(2, 6);
        assert!(&v == &vector[1, 2, 6, 3, 4, 5], 1);

        v.insert(6, 7);
        assert!(&v == &vector[1, 2, 6, 3, 4, 5, 7], 1);

        let v: vector<u64> = vector[];
        v.insert(0, 1);
        assert!(&v == &vector[1], 1);

        let v: vector<u64> = vector[1];
        v.insert(0, 2);
        assert!(&v == &vector[2, 1], 1);

        let v: vector<u64> = vector[1];
        v.insert(1, 2);
        assert!(&v == &vector[1, 2], 1);

        let v: vector<u64> = vector[1, 2];
        v.insert(0, 3);
        assert!(&v == &vector[3, 1, 2], 1);

        let v: vector<u64> = vector[1, 2];
        v.insert(1, 3);
        assert!(&v == &vector[1, 3, 2], 1);

        let v: vector<u64> = vector[1, 2];
        v.insert(2, 3);
        assert!(&v == &vector[1, 2, 3], 1);

        let v: vector<u64> = vector[1, 2, 3];
        v.insert(0, 4);
        assert!(&v == &vector[4, 1, 2, 3], 1);

        let v: vector<u64> = vector[1, 2, 3, 4];
        v.insert(0, 5);
        assert!(&v == &vector[5, 1, 2, 3, 4], 1);

        let v: vector<u64> = vector[1, 2, 3, 4];
        v.insert(1, 5);
        assert!(&v == &vector[1, 5, 2, 3, 4], 1);

        let v: vector<u64> = vector[1, 2, 3, 4];
        v.insert(2, 5);
        assert!(&v == &vector[1, 2, 5, 3, 4], 1);

        let v: vector<u64> = vector[1, 2, 3, 4];
        v.insert(4, 5);
        assert!(&v == &vector[1, 2, 3, 4, 5], 1);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun test_insert_out_of_bounds() {
        let v: vector<u64> = vector[1, 2, 3, 4, 5];

        v.insert(6, 6);
    }

    #[test]
    fun test_range() {
        let result = vector::range(5, 10);
        assert!(result == vector[5, 6, 7, 8, 9], 1);
    }

    #[test]
    fun test_range_with_step() {
        let result = vector::range_with_step(0, 10, 2);
        assert!(result == vector[0, 2, 4, 6, 8], 1);

        let empty_result = vector::range_with_step(10, 10, 2);
        assert!(empty_result == vector[], 1);

        // Test with `start` greater than `end`
        let reverse_result = vector::range_with_step(10, 0, 2);
        assert!(reverse_result == vector[], 1);
    }

    #[test]
    #[expected_failure(abort_code = V::EINVALID_STEP)]
    fun test_range_with_invalid_step() {
        vector::range_with_step(0, 10, 0);
    }

    #[test]
    fun test_slice() {
        let v = &vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let slice_beginning = v.slice(0, 3);
        assert!(slice_beginning == vector[0, 1, 2], 1);

        let slice_end = v.slice(7, 10);
        assert!(slice_end == vector[7, 8, 9], 1);

        let empty_slice = v.slice(5, 5);
        assert!(empty_slice == vector[], 1);
        let empty_slice = v.slice(0, 0);
        assert!(empty_slice == vector[], 1);

        let full_slice = &v.slice(0, 10);
        assert!(full_slice == v, 1);
    }

    #[test]
    #[expected_failure(abort_code = V::EINVALID_SLICE_RANGE)]
    fun test_slice_invalid_range() {
        let v = &vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        v.slice(7, 6); // start is greater than end
    }

    #[test]
    #[expected_failure(abort_code = V::EINVALID_SLICE_RANGE)]
    fun test_slice_out_of_bounds() {
        let v = &vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        v.slice(0, 11); // end is out of bounds
    }

    #[test_only]
    struct MoveOnly {}

    #[test]
    fun test_destroy() {
        let v = vector[MoveOnly {}];
        v.destroy(|m| { let MoveOnly {} = m; })
    }

    #[test]
    fun test_move_range_ints() {
        let v = vector[3, 4, 5, 6];
        let w = vector[1, 2];

        V::move_range(&mut v, 1, 2, &mut w, 1);
        assert!(&v == &vector[3, 6], 0);
        assert!(&w == &vector[1, 4, 5, 2], 0);
    }

    #[test]
    #[expected_failure(abort_code = V::EINDEX_OUT_OF_BOUNDS)]
    fun test_replace_empty_abort() {
        let v = vector[];
        let MoveOnly {} = v.replace(0, MoveOnly {});
        v.destroy_empty();
    }

    #[test]
    fun test_replace() {
        let v = vector[1, 2, 3, 4];
        v.replace(1, 17);
        assert!(v == vector[1, 17, 3, 4], 0);
    }
}
