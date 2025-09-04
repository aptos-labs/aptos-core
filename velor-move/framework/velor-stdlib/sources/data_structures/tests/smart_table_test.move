#[test_only]
module velor_std::smart_table_test {
    use velor_std::smart_table::{Self, SmartTable};

    #[test_only]
    public fun make_smart_table(): SmartTable<u64, u64> {
        let table = smart_table::new_with_config<u64, u64>(0, 50, 10);
        for (i in 0..100) {
            table.add(i, i);
        };
        table
    }

    #[test]
    public fun smart_table_for_each_ref_test() {
        let t = make_smart_table();
        let s = 0;
        t.for_each_ref(|x, y| {
            s += *x + *y;
        });
        assert!(s == 9900, 0);
        t.destroy();
    }

    #[test]
    public fun smart_table_for_each_mut_test() {
        let t = make_smart_table();
        t.for_each_mut(|_key, val| {
            let val: &mut u64 = val;
            *val += 1
        });
        t.for_each_ref(|key, val| {
            assert!(*key + 1 == *val, *key);
        });
        t.destroy();
    }

    #[test]
    public fun smart_table_test_map_ref_test() {
        let t = make_smart_table();
        let r = t.map_ref(|val| *val + 1);
        r.for_each_ref(|key, val| {
            assert!(*key + 1 == *val, *key);
        });
        t.destroy();
        r.destroy();
    }

    #[test]
    public fun smart_table_any_test() {
        let t = make_smart_table();
        let r = t.any(|_k, v| *v >= 99);
        assert!(r, 0);
        t.destroy();
    }
}
