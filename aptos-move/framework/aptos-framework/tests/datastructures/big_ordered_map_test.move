#[test_only]
module aptos_framework::big_ordered_map_test {
    use std::big_ordered_map::{new, new_from, new_with_config};
    use std::option;
    use aptos_std::ordered_map;

    #[test]
    fun test_small_example() {
        let map = new_with_config(5, 3, true);
        map.allocate_spare_slots(2);
        map.print_map(); map.validate_map();
        map.add(1, 1); map.print_map(); map.validate_map();
        map.add(2, 2); map.print_map(); map.validate_map();
        let r1 = map.upsert(3, 3); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 1);
        map.add(4, 4); map.print_map(); map.validate_map();
        let r2 = map.upsert(4, 8); map.print_map(); map.validate_map();
        assert!(r2 == option::some(4), 2);
        map.add(5, 5); map.print_map(); map.validate_map();
        map.add(6, 6); map.print_map(); map.validate_map();

        let expected_keys = vector[1, 2, 3, 4, 5, 6];
        let expected_values = vector[1, 2, 3, 8, 5, 6];

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(k == expected_keys.borrow(index), *k + 100);
            assert!(v == expected_values.borrow(index), *k + 200);
            index += 1;
        });

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(k == expected_keys.borrow(index), *k + 100);
            assert!(v == expected_values.borrow(index), *k + 200);
            index += 1;
        });

        expected_keys.zip(expected_values, |key, value| {
            assert!(map.borrow(&key) == &value, key + 300);
            assert!(map.borrow_mut(&key) == &value, key + 400);
        });

        map.remove(&5); map.print_map(); map.validate_map();
        map.remove(&4); map.print_map(); map.validate_map();
        map.remove(&1); map.print_map(); map.validate_map();
        map.remove(&3); map.print_map(); map.validate_map();
        map.remove(&2); map.print_map(); map.validate_map();
        map.remove(&6); map.print_map(); map.validate_map();

        map.destroy_empty();
    }

    #[test]
    fun test_for_each() {
        let map = new_with_config<u64, u64>(4, 3, false);
        map.add_all(vector[1, 3, 6, 2, 9, 5, 7, 4, 8], vector[1, 3, 6, 2, 9, 5, 7, 4, 8]);

        let expected = vector[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let index = 0;
        map.for_each(|k, v| {
            assert!(k == expected[index], k + 100);
            assert!(v == expected[index], k + 200);
            index += 1;
        });
    }

    #[test]
    fun test_for_each_ref() {
        let map = new_with_config<u64, u64>(4, 3, false);
        map.add_all(vector[1, 3, 6, 2, 9, 5, 7, 4, 8], vector[1, 3, 6, 2, 9, 5, 7, 4, 8]);

        let expected = vector[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(*k == expected[index], *k + 100);
            assert!(*v == expected[index], *k + 200);
            index += 1;
        });

        map.destroy(|_v| {});
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
    fun test_zip_for_each_ref() {
        let map1 = new_with_config<u64, u64>(4, 3, false);
        map1.add_all(vector[1, 2, 4, 8, 9], vector[1, 2, 4, 8, 9]);

        let map2 = new_with_config<u64, u64>(4, 3, false);
        map2.add_all(vector[2, 3, 4, 6, 8, 10, 12, 14], vector[2, 3, 4, 6, 8, 10, 12, 14]);

        let result = new();
        map1.intersection_zip_for_each_ref(&map2, |k, v1, v2| {
            assert!(v1 == v2);
            result.upsert(*k, *v1);
        });

        let result_ordered = result.to_ordered_map();
        let expected_ordered = ordered_map::new_from(vector[2, 4, 8], vector[2, 4, 8]);
        result_ordered.print_map();
        expected_ordered.print_map();
        assert!(expected_ordered == result_ordered);

        let map_empty = new_with_config<u64, u64>(4, 3, false);
        map1.intersection_zip_for_each_ref(&map_empty, |_k, _v1, _v2| {
            abort 1;
        });

        map_empty.intersection_zip_for_each_ref(&map2, |_k, _v1, _v2| {
            abort 1;
        });

        map1.destroy(|_v| {});
        map2.destroy(|_v| {});
        result.destroy(|_v| {});
        map_empty.destroy_empty();
    }

    #[test]
    fun test_variable_size() {
        let map = new_with_config<vector<u64>, vector<u64>>(0, 0, false);
        map.print_map(); map.validate_map();
        map.add(vector[1], vector[1]); map.print_map(); map.validate_map();
        map.add(vector[2], vector[2]); map.print_map(); map.validate_map();
        let r1 = map.upsert(vector[3], vector[3]); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 1);
        map.add(vector[4], vector[4]); map.print_map(); map.validate_map();
        let r2 = map.upsert(vector[4], vector[8, 8, 8]); map.print_map(); map.validate_map();
        assert!(r2 == option::some(vector[4]), 2);
        map.add(vector[5], vector[5]); map.print_map(); map.validate_map();
        map.add(vector[6], vector[6]); map.print_map(); map.validate_map();

        vector[1, 2, 3, 4, 5, 6].zip(vector[1, 2, 3, 8, 5, 6], |key, value| {
            assert!(map.borrow(&vector[key])[0] == value, key + 100);
        });

        map.remove(&vector[5]); map.print_map(); map.validate_map();
        map.remove(&vector[4]); map.print_map(); map.validate_map();
        map.remove(&vector[1]); map.print_map(); map.validate_map();
        map.remove(&vector[3]); map.print_map(); map.validate_map();
        map.remove(&vector[2]); map.print_map(); map.validate_map();
        map.remove(&vector[6]); map.print_map(); map.validate_map();

        map.destroy_empty();
    }
    #[test]
    fun test_deleting_and_creating_nodes() {
        let map = new_with_config(4, 3, true);
        map.allocate_spare_slots(2);

        for (i in 0..25) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 0..20) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 25..50) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 25..45) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 50..75) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 50..75) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 20..25) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 45..50) {
            map.remove(&i);
            map.validate_map();
        };

        map.destroy_empty();
    }

    #[test]
    fun test_iterator() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[1, 7, 5, 8, 4, 2, 6, 3, 9, 0];
        while (data.length() != 0) {
            let element = data.pop_back();
            map.add(element, element);
        };

        let it = map.internal_new_begin_iter();

        let i = 0;
        while (!it.iter_is_end(&map)) {
            assert!(it.iter_borrow_key() == &i, i);
            assert!(it.iter_borrow(&map) == &i, i);
            assert!(it.iter_borrow_mut(&mut map) == &i, i);
            i += 1;
            it = it.iter_next(&map);
        };

        map.destroy(|_v| {});
    }

    #[test]
    fun test_find() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 0, 10];
        map.add_all(data, data);

        let i = 0;
        while (i < data.length()) {
            let element = data.borrow(i);
            let it = map.internal_find(element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_borrow_key() == element, i);
            i += 1;
        };

        assert!(map.internal_find(&4).iter_is_end(&map), 0);
        assert!(map.internal_find(&9).iter_is_end(&map), 1);

        map.destroy(|_v| {});
    }

    #[test]
    fun test_lower_bound() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 12, 10];
        map.add_all(data, data);

        let i = 0;
        while (i < data.length()) {
            let element = data[i];
            let it = map.internal_lower_bound(&element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_borrow_key() == &element, i);
            i += 1;
        };

        assert!(map.internal_lower_bound(&0).iter_borrow_key() == &1, 0);
        assert!(map.internal_lower_bound(&4).iter_borrow_key() == &5, 1);
        assert!(map.internal_lower_bound(&9).iter_borrow_key() == &10, 2);
        assert!(map.internal_lower_bound(&13).iter_is_end(&map), 3);

        map.remove(&3);
        assert!(map.internal_lower_bound(&3).iter_borrow_key() == &5, 4);
        map.remove(&5);
        assert!(map.internal_lower_bound(&3).iter_borrow_key() == &6, 5);
        assert!(map.internal_lower_bound(&4).iter_borrow_key() == &6, 6);

        map.destroy(|_v| {});
    }

    #[test]
    fun test_modify_and_get() {
        let map = new_with_config(4, 3, false);
        map.add_all(vector[1, 2, 3], vector[1, 2, 3]);
        map.modify(&2, |v| *v += 10);
        assert!(map.get(&2) == option::some(12));
        assert!(map.get(&4) == option::none());

        assert!(map.get_and_map(&2, |v| *v + 5) == option::some(17));
        assert!(map.get_and_map(&4, |v| *v + 5) == option::none());

        map.modify_or_add(&3, |v| *v += 10, || 20);
        assert!(map.get(&3) == option::some(13));
        map.modify_or_add(&4, |v| *v += 10, || 20);
        assert!(map.get(&4) == option::some(20));

        assert!(option::some(7) == map.modify_if_present_and_return(&4, |v| { *v += 10; 7}));
        assert!(map.get(&4) == option::some(30));

        assert!(option::none() == map.modify_if_present_and_return(&5, |v| { *v += 10; 7}));

        map.destroy(|_v| {});
    }

    #[test]
    fun test_contains() {
        let map = new_with_config(4, 3, false);
        let data = vector[3, 1, 9, 7, 5];
        map.add_all(vector[3, 1, 9, 7, 5], vector[3, 1, 9, 7, 5]);

        data.for_each_ref(|i| assert!(map.contains(i), *i));

        let missing = vector[0, 2, 4, 6, 8, 10];
        missing.for_each_ref(|i| assert!(!map.contains(i), *i));

        map.destroy(|_v| {});
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
        assert!(front_k == 1, 6);
        assert!(front_v == &10, 7);

        let (back_k, back_v) = map.borrow_back();
        assert!(back_k == 3, 8);
        assert!(back_v == &30, 9);

        let (front_k, front_v) = map.pop_front();
        assert!(front_k == 1, 10);
        assert!(front_v == 10, 11);

        let (back_k, back_v) = map.pop_back();
        assert!(back_k == 3, 12);
        assert!(back_v == 30, 13);

        map.destroy(|_v| {});
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = aptos_framework::big_ordered_map)] /// EINVALID_CONFIG_PARAMETER
    fun test_inner_max_degree_too_large() {
        let map = new_with_config<u8, u8>(4097, 0, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = aptos_framework::big_ordered_map)] /// EINVALID_CONFIG_PARAMETER
    fun test_inner_max_degree_too_small() {
        let map = new_with_config<u8, u8>(3, 0, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = aptos_framework::big_ordered_map)] /// EINVALID_CONFIG_PARAMETER
    fun test_leaf_max_degree_too_small() {
        let map = new_with_config<u8, u8>(0, 2, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = aptos_framework::big_ordered_map)] /// EKEY_ALREADY_EXISTS
    fun test_abort_add_existing_value() {
        let map = new_from(vector[1], vector[1]);
        map.add(1, 2);
        map.destroy_and_validate();
    }

    #[test_only]
    fun vector_range(from: u64, to: u64): vector<u64> {
        let result = vector[];
        for (i in from..to) {
            result.push_back(i);
        };
        result
    }

    #[test_only]
    fun vector_bytes_range(from: u64, to: u64): vector<u8> {
        let result = vector[];
        for (i in from..to) {
            result.push_back((i % 128) as u8);
        };
        result
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = aptos_framework::big_ordered_map)] /// EKEY_ALREADY_EXISTS
    fun test_abort_add_existing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.add(3, 3);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_std::ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_remove_missing_value() {
        let map = new_from(vector[1], vector[1]);
        map.remove(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_std::ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_remove_missing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.remove(&4);
        map.remove(&4);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_framework::big_ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_remove_largest_missing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.remove(&11);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_framework::big_ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_borrow_missing() {
        let map = new_from(vector[1], vector[1]);
        map.borrow(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_framework::big_ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_borrow_mut_missing() {
        let map = new_from(vector[1], vector[1]);
        map.borrow_mut(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000E, location = aptos_framework::big_ordered_map)] /// EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE
    fun test_abort_borrow_mut_requires_constant_value_size() {
        let map = new_with_config(0, 0, false);
        map.add(1, vector[1]);
        map.borrow_mut(&1);
        map.destroy_and_validate();
    }

    #[test]
    fun test_borrow_mut_allows_variable_key_size() {
        let map = new_with_config(0, 0, false);
        map.add(vector[1], 1);
        map.borrow_mut(&vector[1]);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::big_ordered_map)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_key_missing() {
        let map = new_from(vector[1], vector[1]);
        map.internal_new_end_iter().iter_borrow_key();
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::big_ordered_map)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_missing() {
        let map = new_from(vector[1], vector[1]);
        map.internal_new_end_iter().iter_borrow(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::big_ordered_map)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_mut_missing() {
        let map = new_from(vector[1], vector[1]);
        map.internal_new_end_iter().iter_borrow_mut(&mut map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000E, location = aptos_framework::big_ordered_map)] /// EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE
    fun test_abort_iter_borrow_mut_requires_constant_kv_size() {
        let map = new_with_config(0, 0, false);
        map.add(1, vector[1]);
        map.internal_new_begin_iter().iter_borrow_mut(&mut map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::big_ordered_map)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_end_iter_next() {
        let map = new_from(vector[1, 2, 3], vector[1, 2, 3]);
        map.internal_new_end_iter().iter_next(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::big_ordered_map)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_begin_iter_prev() {
        let map = new_from(vector[1, 2, 3], vector[1, 2, 3]);
        map.internal_new_begin_iter().iter_prev(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000C, location = aptos_framework::big_ordered_map)] /// EMAP_NOT_EMPTY
    fun test_abort_fail_to_destroy_non_empty() {
        let map = new_from(vector[1], vector[1]);
        map.destroy_empty();
    }

    #[test]
    fun test_default_allows_5kb() {
        let map = new_with_config(0, 0, false);
        map.add(vector[1u8], 1);
        // default guarantees key up to 5KB
        map.add(vector_bytes_range(0, 5000), 1);
        map.destroy_and_validate();

        let map = new_with_config(0, 0, false);
        // default guarantees (key, value) pair up to 10KB
        map.add(1, vector[1u8]);
        map.add(2, vector_bytes_range(0, 10000));
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000F, location = aptos_framework::big_ordered_map)] /// EKEY_BYTES_TOO_LARGE
    fun test_adding_key_too_large() {
        let map = new_with_config(0, 0, false);
        map.add(vector[1u8], 1);
        // default guarantees key up to 5KB
        map.add(vector_bytes_range(0, 5200), 1);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000D, location = aptos_framework::big_ordered_map)] /// EARGUMENT_BYTES_TOO_LARGE
    fun test_adding_value_too_large() {
        let map = new_with_config(0, 0, false);
        // default guarantees (key, value) pair up to 10KB
        map.add(1, vector[1u8]);
        map.add(2, vector_bytes_range(0, 12000));
        map.destroy_and_validate();
    }

    #[test_only]
    inline fun comparison_test(repeats: u64, inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, next_1: ||u64, next_2: ||u64) {
        let big_map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots);
        if (reuse_slots) {
            big_map.allocate_spare_slots(4);
        };
        let small_map = ordered_map::new();
        for (i in 0..repeats) {
            let is_insert = if (2 * i < repeats) {
                i % 3 != 2
            } else {
                i % 3 == 0
            };
            if (is_insert) {
                let v = next_1();
                assert!(big_map.upsert(v, v) == small_map.upsert(v, v), i);
            } else {
                let v = next_2();
                assert!(big_map.remove(&v) == small_map.remove(&v), i);
            };
            if ((i + 1) % 50 == 0) {
                big_map.validate_map();

                let big_iter = big_map.internal_new_begin_iter();
                let small_iter = small_map.internal_new_begin_iter();
                while (!big_iter.iter_is_end(&big_map) || !small_iter.iter_is_end(&small_map)) {
                    assert!(big_iter.iter_borrow_key() == small_iter.iter_borrow_key(&small_map), i);
                    assert!(big_iter.iter_borrow(&big_map) == small_iter.iter_borrow(&small_map), i);
                    big_iter = big_iter.iter_next(&big_map);
                    small_iter = small_iter.iter_next(&small_map);
                };
            };
        };
        big_map.destroy_and_validate();
    }

    #[test_only]
    const OFFSET: u64 = 270001;
    #[test_only]
    const MOD: u64 = 1000000;

    #[test]
    fun test_comparison_random() {
        let x = 1234;
        let y = 1234;
        comparison_test(500, 5, 5, false,
            || {
                x += OFFSET;
                if (x > MOD) { x -= MOD};
                x
            },
            || {
                y += OFFSET;
                if (y > MOD) { y -= MOD};
                y
            },
        );
    }

    #[test]
    fun test_comparison_increasing() {
        let x = 0;
        let y = 0;
        comparison_test(500, 5, 5, false,
            || {
                x += 1;
                x
            },
            || {
                y += 1;
                y
            },
        );
    }

    #[test]
    fun test_comparison_decreasing() {
        let x = 100000;
        let y = 100000;
        comparison_test(500, 5, 5, false,
            || {
                x -= 1;
                x
            },
            || {
                y -= 1;
                y
            },
        );
    }

    #[test_only]
    fun test_large_data_set_helper(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool) {
        use std::vector;

        let map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots);
        if (reuse_slots) {
            map.allocate_spare_slots(4);
        };
        let data = ordered_map::large_dataset();
        let shuffled_data = ordered_map::large_dataset_shuffled();

        let len = data.length();
        for (i in 0..len) {
            let element = data[i];
            map.upsert(element, element);
            if (i % 7 == 0) {
                map.validate_map();
            }
        };

        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            let it = map.internal_find(element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_borrow_key() == element, i);

            // aptos_std::debug::print(&it);

            let it_next = it.iter_next(&map);
            let it_after = map.internal_lower_bound(&(*element + 1));

            // aptos_std::debug::print(&it_next);
            // aptos_std::debug::print(&it_after);
            // aptos_std::debug::print(&std::string::utf8(b"bla"));

            assert!(it_next == it_after, i);
        };

        let removed = vector::empty();
        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            if (!removed.contains(element)) {
                removed.push_back(*element);
                map.remove(element);
                if (i % 7 == 1) {
                    map.validate_map();

                }
            } else {
                assert!(!map.contains(element));
            };
        };

        map.destroy_empty();
    }

    // Currently ignored long / more extensive tests.

    // #[test]
    // fun test_large_data_set_order_5_false() {
    //     test_large_data_set_helper(5, 5, false);
    // }

    // #[test]
    // fun test_large_data_set_order_5_true() {
    //     test_large_data_set_helper(5, 5, true);
    // }

    // #[test]
    // fun test_large_data_set_order_4_3_false() {
    //     test_large_data_set_helper(4, 3, false);
    // }

    // #[test]
    // fun test_large_data_set_order_4_3_true() {
    //     test_large_data_set_helper(4, 3, true);
    // }

    // #[test]
    // fun test_large_data_set_order_4_4_false() {
    //     test_large_data_set_helper(4, 4, false);
    // }

    // #[test]
    // fun test_large_data_set_order_4_4_true() {
    //     test_large_data_set_helper(4, 4, true);
    // }

    // #[test]
    // fun test_large_data_set_order_6_false() {
    //     test_large_data_set_helper(6, 6, false);
    // }

    // #[test]
    // fun test_large_data_set_order_6_true() {
    //     test_large_data_set_helper(6, 6, true);
    // }

    // #[test]
    // fun test_large_data_set_order_6_3_false() {
    //     test_large_data_set_helper(6, 3, false);
    // }

    #[test]
    fun test_large_data_set_order_6_3_true() {
        test_large_data_set_helper(6, 3, true);
    }

    #[test]
    fun test_large_data_set_order_4_6_false() {
        test_large_data_set_helper(4, 6, false);
    }

    // #[test]
    // fun test_large_data_set_order_4_6_true() {
    //     test_large_data_set_helper(4, 6, true);
    // }

    // #[test]
    // fun test_large_data_set_order_16_false() {
    //     test_large_data_set_helper(16, 16, false);
    // }

    // #[test]
    // fun test_large_data_set_order_16_true() {
    //     test_large_data_set_helper(16, 16, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_false() {
    //     test_large_data_set_helper(31, 31, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_true() {
    //     test_large_data_set_helper(31, 31, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_3_false() {
    //     test_large_data_set_helper(31, 3, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_3_true() {
    //     test_large_data_set_helper(31, 3, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_5_false() {
    //     test_large_data_set_helper(31, 5, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_5_true() {
    //     test_large_data_set_helper(31, 5, true);
    // }

    // #[test]
    // fun test_large_data_set_order_32_false() {
    //     test_large_data_set_helper(32, 32, false);
    // }

    // #[test]
    // fun test_large_data_set_order_32_true() {
    //     test_large_data_set_helper(32, 32, true);
    // }

}
