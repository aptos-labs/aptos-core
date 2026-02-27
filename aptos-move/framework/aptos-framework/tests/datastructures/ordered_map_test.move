#[test_only]
module aptos_framework::ordered_map_test {
    use std::ordered_map::{new, new_from};
    use std::option;

    #[test]
    fun test_map_small() {
        let map = new();
        map.validate_map();
        map.add(1, 1);
        map.validate_map();
        map.add(2, 2);
        map.validate_map();
        let r1 = map.upsert(3, 3);
        map.validate_map();
        assert!(r1 == option::none(), 4);
        map.add(4, 4);
        map.validate_map();
        let r2 = map.upsert(4, 8);
        map.validate_map();
        assert!(r2 == option::some(4), 5);
        map.add(5, 5);
        map.validate_map();
        map.add(6, 6);
        map.validate_map();

        map.remove(&5);
        map.validate_map();
        map.remove(&4);
        map.validate_map();
        map.remove(&1);
        map.validate_map();
        map.remove(&3);
        map.validate_map();
        map.remove(&2);
        map.validate_map();
        map.remove(&6);
        map.validate_map();

        map.destroy_empty();
    }

    #[test]
    fun test_add_remove_many() {
        let map = new<u64, u64>();

        assert!(map.length() == 0, 0);
        assert!(!map.contains(&3), 1);
        map.add(3, 1);
        assert!(map.length() == 1, 2);
        assert!(map.contains(&3), 3);
        assert!(map.borrow(&3) == &1, 4);
        *map.borrow_mut(&3) = 2;
        assert!(map.borrow(&3) == &2, 5);

        assert!(!map.contains(&2), 6);
        map.add(2, 5);
        assert!(map.length() == 2, 7);
        assert!(map.contains(&2), 8);
        assert!(map.borrow(&2) == &5, 9);
        *map.borrow_mut(&2) = 9;
        assert!(map.borrow(&2) == &9, 10);

        map.remove(&2);
        assert!(map.length() == 1, 11);
        assert!(!map.contains(&2), 12);
        assert!(map.borrow(&3) == &2, 13);

        map.remove(&3);
        assert!(map.length() == 0, 14);
        assert!(!map.contains(&3), 15);

        map.destroy_empty();
    }

    #[test]
    fun test_add_all() {
        let map = new<u64, u64>();

        assert!(map.length() == 0, 0);
        map.add_all(vector[2, 1, 3], vector[20, 10, 30]);

        assert!(map == new_from(vector[1, 2, 3], vector[10, 20, 30]), 1);

        assert!(map.length() == 3, 1);
        assert!(map.borrow(&1) == &10, 2);
        assert!(map.borrow(&2) == &20, 3);
        assert!(map.borrow(&3) == &30, 4);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_add_all_mismatch() {
        new_from(vector[1, 3], vector[10]);
    }

    #[test]
    fun test_upsert_all() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.upsert_all(vector[7, 2, 3], vector[70, 20, 35]);
        assert!(map == new_from(vector[1, 2, 3, 5, 7], vector[10, 20, 35, 50, 70]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_new_from_duplicate() {
        new_from(vector[1, 3, 1, 5], vector[10, 30, 11, 50]);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_upsert_all_mismatch() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.upsert_all(vector[2], vector[20, 35]);
    }

    #[test]
    fun test_to_vec_pair() {
        let (keys, values) = new_from(vector[3, 1, 5], vector[30, 10, 50]).to_vec_pair();
        assert!(keys == vector[1, 3, 5], 1);
        assert!(values == vector[10, 30, 50], 2);
    }

    #[test]
    fun test_keys() {
        let map = new<u64, u64>();
        assert!(map.keys() == vector[], 0);
        map.add(2, 1);
        map.add(3, 1);

        assert!(map.keys() == vector[2, 3], 0);
    }

    #[test]
    fun test_values() {
        let map = new<u64, u64>();
        assert!(map.values() == vector[], 0);
        map.add(2, 1);
        map.add(3, 2);

        assert!(map.values() == vector[1, 2], 0);
    }

    #[test]
    fun test_modify_and_get() {
        let map = new<u64, u64>();
        map.add_all(vector[1, 2, 3], vector[1, 2, 3]);
        assert!(true == map.modify_if_present(&2, |v| *v += 10));
        assert!(map.get(&2) == option::some(12));
        assert!(map.get(&4) == option::none());

        assert!(map.get_and_map(&2, |v| *v + 5) == option::some(17));
        assert!(map.get_and_map(&4, |v| *v + 5) == option::none());

        map.modify_or_add(&3, |v| {*v += 10}, || 20);
        assert!(map.get(&3) == option::some(13));
        map.modify_or_add(&4, |v| {*v += 10}, || 20);
        assert!(map.get(&4) == option::some(20));
    }

    #[test]
    fun test_for_each_variants() {
        let keys = vector[1, 3, 5];
        let values = vector[10, 30, 50];
        let map = new_from(keys, values);

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            index += 1;
        });

        let index = 0;
        map.for_each_mut(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            *v += 1;
            index += 1;
        });

        let index = 0;
        map.for_each(|k, v| {
            assert!(keys[index] == k);
            assert!(values[index] + 1 == v);
            index += 1;
        });
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_add_twice() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.add(3, 1);

        map.remove(&3);
        map.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_remove_twice_1() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.remove(&3);
        map.remove(&3);

        map.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_remove_twice_2() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.add(4, 1);
        map.remove(&3);
        map.remove(&3);

        map.destroy_empty();
    }

    #[test]
    fun test_upsert_test() {
        let map = new<u64, u64>();
        // test adding 3 elements using upsert
        map.upsert<u64, u64>(1, 1);
        map.upsert(2, 2);
        map.upsert(3, 3);

        assert!(map.length() == 3, 0);
        assert!(map.contains(&1), 1);
        assert!(map.contains(&2), 2);
        assert!(map.contains(&3), 3);
        assert!(map.borrow(&1) == &1, 4);
        assert!(map.borrow(&2) == &2, 5);
        assert!(map.borrow(&3) == &3, 6);

        // change mapping 1->1 to 1->4
        map.upsert(1, 4);

        assert!(map.length() == 3, 7);
        assert!(map.contains(&1), 8);
        assert!(map.borrow(&1) == &4, 9);
    }

    #[test]
    fun test_append() {
        {
            let map = new<u16, u16>();
            let other = new();
            map.append(other);
            assert!(map.is_empty(), 0);
        };
        {
            let map = new_from(vector[1, 2], vector[10, 20]);
            let other = new();
            map.append(other);
            assert!(map == new_from(vector[1, 2], vector[10, 20]), 1);
        };
        {
            let map = new();
            let other = new_from(vector[1, 2], vector[10, 20]);
            map.append(other);
            assert!(map == new_from(vector[1, 2], vector[10, 20]), 2);
        };
        {
            let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
            let other = new_from(vector[4, 5], vector[40, 50]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 3);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            let other = new_from(vector[2, 4], vector[20, 40]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 4);
        };
        {
            let map = new_from(vector[2, 4], vector[20, 40]);
            let other = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 6);
        };
        {
            let map = new_from(vector[1], vector[10]);
            let other = new_from(vector[1], vector[11]);
            map.append(other);
            assert!(map == new_from(vector[1], vector[11]), 7);
        }
    }

    #[test]
    fun test_append_disjoint() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        let other = new_from(vector[4, 5], vector[40, 50]);
        map.append_disjoint(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_append_disjoint_abort() {
        let map = new_from(vector[1], vector[10]);
        let other = new_from(vector[1], vector[11]);
        map.append_disjoint(other);
    }

    #[test]
    fun test_trim() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        let rest = map.trim(2);
        assert!(map == new_from(vector[1, 2], vector[10, 20]), 1);
        assert!(rest == new_from(vector[3], vector[30]), 2);
    }

    #[test]
    fun test_non_iterator_ordering() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        assert!(map.prev_key(&1).is_none(), 1);
        assert!(map.next_key(&1) == option::some(2), 1);

        assert!(map.prev_key(&2) == option::some(1), 2);
        assert!(map.next_key(&2) == option::some(3), 3);

        assert!(map.prev_key(&3) == option::some(2), 4);
        assert!(map.next_key(&3).is_none(), 5);

        let (front_k, front_v) = map.borrow_front();
        assert!(front_k == &1, 6);
        assert!(front_v == &10, 7);

        let (back_k, back_v) = map.borrow_back();
        assert!(back_k == &3, 8);
        assert!(back_v == &30, 9);

        let (front_k, front_v) = map.pop_front();
        assert!(front_k == 1, 10);
        assert!(front_v == 10, 11);

        let (back_k, back_v) = map.pop_back();
        assert!(back_k == 3, 12);
        assert!(back_v == 30, 13);
    }

    #[test]
    fun test_replace_key_inplace() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.replace_key_inplace(&5, 6);
        assert!(map == new_from(vector[1, 3, 6], vector[10, 30, 50]), 1);
        map.replace_key_inplace(&3, 4);
        assert!(map == new_from(vector[1, 4, 6], vector[10, 30, 50]), 2);
        map.replace_key_inplace(&1, 0);
        assert!(map == new_from(vector[0, 4, 6], vector[10, 30, 50]), 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_replace_key_inplace_not_found_1() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&4, 5);

    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_replace_key_inplace_not_found_2() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&7, 8);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_1() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&3, 7);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_2() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&1, 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_3() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&6, 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_next_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_next(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_key_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_borrow_key(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_borrow(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_mut_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_borrow_mut(&mut map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_begin_prev_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_prev(&map);
    }

    #[test]
    public fun test_iter_is_begin_from_non_empty() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        let iter = map.internal_new_begin_iter();
        assert!(iter.iter_is_begin(&map), 1);
        assert!(iter.iter_is_begin_from_non_empty(), 1);

        iter = iter.iter_next(&map);
        assert!(!iter.iter_is_begin(&map), 1);
        assert!(!iter.iter_is_begin_from_non_empty(), 1);

        let map = new<u64, u64>();
        let iter = map.internal_new_begin_iter();
        assert!(iter.iter_is_begin(&map), 1);
        assert!(!iter.iter_is_begin_from_non_empty(), 1);
    }

    #[test]
    public fun test_iter_remove() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_next(&map).iter_remove(&mut map);
        assert!(map == new_from(vector[1, 5], vector[10, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
        public fun test_iter_remove_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_remove(&mut map);
    }

    #[test]
    public fun test_iter_replace() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_next(&map).iter_replace(&mut map, 35);
        assert!(map == new_from(vector[1, 3, 5], vector[10, 35, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
        public fun test_iter_replace_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_replace(&mut map, 35);
    }

    #[test]
    public fun test_iter_add() {
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.internal_new_begin_iter().iter_add(&mut map, 0, 5);
            assert!(map == new_from(vector[0, 1, 3, 5], vector[5, 10, 30, 50]), 1);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.internal_new_begin_iter().iter_next(&map).iter_add(&mut map, 2, 20);
            assert!(map == new_from(vector[1, 2, 3, 5], vector[10, 20, 30, 50]), 2);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.internal_new_end_iter().iter_add(&mut map, 6, 60);
            assert!(map == new_from(vector[1, 3, 5, 6], vector[10, 30, 50, 60]), 3);
        };
        {
            let map = new();
            map.internal_new_end_iter().iter_add(&mut map, 1, 10);
            assert!(map == new_from(vector[1], vector[10]), 4);
        };
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_1() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_add(&mut map, 1, 5);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_2() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_end_iter().iter_add(&mut map, 5, 55);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_3() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_next(&map).iter_add(&mut map, 1, 15);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_4() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.internal_new_begin_iter().iter_next(&map).iter_add(&mut map, 3, 25);
    }

    #[test]
	public fun test_ordered_map_append_2() {
        let map = new_from(vector[1, 2], vector[10, 20]);
        let other = new_from(vector[1, 2], vector[100, 200]);
        map.append(other);
        assert!(map == new_from(vector[1, 2], vector[100, 200]));
    }

    #[test]
	public fun test_ordered_map_append_3() {
        let map = new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]);
        let other = new_from(vector[2, 4], vector[200, 400]);
        map.append(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 200, 30, 400, 50]));
    }

    #[test]
	public fun test_ordered_map_append_4() {
        let map = new_from(vector[3, 4, 5, 6, 7], vector[30, 40, 50, 60, 70]);
        let other = new_from(vector[1, 2, 4, 6], vector[100, 200, 400, 600]);
        map.append(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5, 6, 7], vector[100, 200, 30, 400, 50, 600, 70]));
    }

    #[test]
	public fun test_ordered_map_append_5() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        let other = new_from(vector[0, 2, 4, 6], vector[0, 200, 400, 600]);
        map.append(other);
        aptos_std::debug::print(&map);
        assert!(map == new_from(vector[0, 1, 2, 3, 4, 5, 6], vector[0, 10, 200, 30, 400, 50, 600]));
    }

    #[test_only]
    public fun large_dataset(): vector<u64> {
        vector[383, 886, 777, 915, 793, 335, 386, 492, 649, 421, 362, 27, 690, 59, 763, 926, 540, 426, 172, 736, 211, 368, 567, 429, 782, 530, 862, 123, 67, 135, 929, 802, 22, 58, 69, 167, 393, 456, 11, 42, 229, 373, 421, 919, 784, 537, 198, 324, 315, 370, 413, 526, 91, 980, 956, 873, 862, 170, 996, 281, 305, 925, 84, 327, 336, 505, 846, 729, 313, 857, 124, 895, 582, 545, 814, 367, 434, 364, 43, 750, 87, 808, 276, 178, 788, 584, 403, 651, 754, 399, 932, 60, 676, 368, 739, 12, 226, 586, 94, 539, 795, 570, 434, 378, 467, 601, 97, 902, 317, 492, 652, 756, 301, 280, 286, 441, 865, 689, 444, 619, 440, 729, 31, 117, 97, 771, 481, 675, 709, 927, 567, 856, 497, 353, 586, 965, 306, 683, 219, 624, 528, 871, 732, 829, 503, 19, 270, 368, 708, 715, 340, 149, 796, 723, 618, 245, 846, 451, 921, 555, 379, 488, 764, 228, 841, 350, 193, 500, 34, 764, 124, 914, 987, 856, 743, 491, 227, 365, 859, 936, 432, 551, 437, 228, 275, 407, 474, 121, 858, 395, 29, 237, 235, 793, 818, 428, 143, 11, 928, 529]
    }

    #[test_only]
    public fun large_dataset_shuffled(): vector<u64> {
        vector[895, 228, 530, 784, 624, 335, 729, 818, 373, 456, 914, 226, 368, 750, 428, 956, 437, 586, 763, 235, 567, 91, 829, 690, 434, 178, 584, 426, 228, 407, 237, 497, 764, 135, 124, 421, 537, 270, 11, 367, 378, 856, 529, 276, 729, 618, 929, 227, 149, 788, 925, 675, 121, 795, 306, 198, 421, 350, 555, 441, 403, 932, 368, 383, 928, 841, 440, 771, 364, 902, 301, 987, 467, 873, 921, 11, 365, 340, 739, 492, 540, 386, 919, 723, 539, 87, 12, 782, 324, 862, 689, 395, 488, 793, 709, 505, 582, 814, 245, 980, 936, 736, 619, 69, 370, 545, 764, 886, 305, 551, 19, 865, 229, 432, 29, 754, 34, 676, 43, 846, 451, 491, 871, 500, 915, 708, 586, 60, 280, 652, 327, 172, 856, 481, 796, 474, 219, 651, 170, 281, 84, 97, 715, 857, 353, 862, 393, 567, 368, 777, 97, 315, 526, 94, 31, 167, 123, 413, 503, 193, 808, 649, 143, 42, 444, 317, 67, 926, 434, 211, 379, 570, 683, 965, 732, 927, 429, 859, 313, 528, 996, 117, 492, 336, 22, 399, 275, 802, 743, 124, 846, 58, 858, 286, 756, 601, 27, 59, 362, 793]
    }

    #[test]
    fun test_map_large() {
        let map = new();
        let data = large_dataset();
        let shuffled_data = large_dataset_shuffled();

        let len = data.length();
        for (i in 0..len) {
            let element = data[i];
            map.upsert(element, element);
            map.validate_map();
        };

        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            let it = map.internal_find(element);
            assert!(!it.iter_is_end(&map), 6);
            assert!(it.iter_borrow_key(&map) == element, 7);

            let it_next = it.iter_next(&map);
            let it_after = map.internal_lower_bound(&(*element + 1));

            assert!(it_next == it_after, 8);
        };

        let removed = vector[];
        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            if (!removed.contains(element)) {
                removed.push_back(*element);
                map.remove(element);
                map.validate_map();
            } else {
                assert!(!map.contains(element));
            };
        };

        map.destroy_empty();
    }


}
