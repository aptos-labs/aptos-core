#[test_only]
module aptos_std::smart_vector_test {
    use aptos_std::smart_vector as V;
    use aptos_std::smart_vector::SmartVector;

    #[test_only]
    fun make_smart_vector(k: u64): SmartVector<u64> {
        let v = V::new<u64>();
        let i = 1u64;
        while (i <= k) {
            v.push_back(i);
            i += 1;
        };
        v
    }

    #[test]
    fun smart_vector_for_each_test() {
        let v = make_smart_vector(100);
        let i = 0;
        v.for_each(|x| {
            assert!(i + 1 == x, 0);
            i += 1;
        });
    }

    #[test]
    fun smart_vector_for_each_reverse_test() {
        let v = make_smart_vector(100);
        let i = 0;
        v.for_each_reverse(|x| {
            assert!(i == 100 - x, 0);
            i += 1;
        });
    }

    #[test]
    fun smart_vector_for_each_ref_test() {
        let v = make_smart_vector(100);
        let s = 0;
        v.for_each_ref(|x| {
            s += *x;
        });
        assert!(s == 5050, 0);
        v.destroy();
    }

    #[test]
    fun smart_vector_for_each_mut_test() {
        let v = make_smart_vector(100);
        v.for_each_mut(|x| {
            let x: &mut u64 = x;
            *x += 1;
        });
        assert!(v.fold(0, |s, x| {
            s + x
        }) == 5150, 0);
    }

    #[test]
    fun smart_vector_enumerate_ref_test() {
        let v = make_smart_vector(100);
        v.enumerate_ref(|i, x| {
            assert!(i + 1 == *x, 0);
        });
        v.destroy();
    }

    #[test]
    fun smart_vector_enumerate_mut_test() {
        let v = make_smart_vector(100);
        v.enumerate_mut(|i, x| {
            let x: &mut u64 = x;
            assert!(i + 1 == *x, 0);
            *x += 1;
        });
        assert!(v.fold(0, |s, x| {
            s + x
        }) == 5150, 0);
    }

    #[test]
    fun smart_vector_fold_test() {
        let v = make_smart_vector(100);
        let i = 0;
        let sum = v.fold(0, |s, x| {
            assert!(i + 1 == x, 0);
            i += 1;
            s + x
        });
        assert!(sum == 5050, 0);
    }

    #[test]
    fun smart_vector_for_foldr_test() {
        let v = make_smart_vector(100);
        let i = 0;
        let sum = v.foldr(0, |x, s| {
            assert!(i == 100 - x, i);
            i += 1;
            s + x
        });
        assert!(sum == 5050, 0);
    }

    #[test]
    fun smart_vector_map_test() {
        let v = make_smart_vector(100);
        let mapped_v = v.map(|x| { x * 2 });
        let sum = mapped_v.fold(0, |s, x| {
            s + x
        });
        assert!(sum == 10100, 0);
    }

    #[test]
    fun smart_vector_map_ref_test() {
        let v = make_smart_vector(100);
        let mapped_v = v.map_ref(|x|  *x * 2);
        assert!(v.fold(0, |s, x| {
            s + x
        }) == 5050, 0);
        assert!(mapped_v.fold(0, |s, x| {
            s + x
        }) == 10100, 0);
    }

    #[test]
    fun smart_vector_filter_test() {
        let v = make_smart_vector(100);
        let filtered_v = v.filter(|x| *x % 10 == 0);
        filtered_v.enumerate_ref(|i, x| {
            assert!((i + 1) * 10 == *x, 0);
        });
        filtered_v.destroy();
    }

    #[test]
    fun smart_vector_test_zip() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(100);
        let s = 0;
        v1.zip(v2, |e1, e2| {
            let e1: u64 = e1;
            let e2: u64 = e2;
            s += e1 / e2
        });
        assert!(s == 100, 0);
    }

    #[test]
    // zip is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::ESMART_VECTORS_LENGTH_MISMATCH, location = Self)]
    fun smart_vector_test_zip_mismatching_lengths_should_fail() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(99);
        let s = 0;
        v1.zip(v2, |e1, e2| {
            let e1: u64 = e1;
            let e2: u64 = e2;
            s += e1 / e2
        });
    }

    #[test]
    fun smart_vector_test_zip_ref() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(100);
        let s = 0;
        v1.zip_ref(&v2, |e1, e2| s += *e1 / *e2);
        assert!(s == 100, 0);
        v1.destroy();
        v2.destroy();
    }

    #[test]
    // zip_ref is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::ESMART_VECTORS_LENGTH_MISMATCH, location = Self)]
    fun smart_vector_test_zip_ref_mismatching_lengths_should_fail() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(99);
        let s = 0;
        v1.zip_ref(&v2, |e1, e2| s += *e1 / *e2);
        v1.destroy();
        v2.destroy();
    }

    #[test]
    fun smart_vector_test_zip_mut() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(100);
        v1.zip_mut(&mut v2, |e1, e2| {
            let e1: &mut u64 = e1;
            let e2: &mut u64 = e2;
            *e1 += 1;
            *e2 -= 1;
        });
        v1.zip_ref(&v2, |e1, e2| assert!(*e1 == *e2 + 2, 0));
        v1.destroy();
        v2.destroy();
    }

    #[test]
    fun smart_vector_test_zip_map() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(100);
        let result = v1.zip_map(v2, |e1, e2| e1 / e2);
        result.for_each(|v| assert!(v == 1, 0));
    }

    #[test]
    fun smart_vector_test_zip_map_ref() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(100);
        let result = v1.zip_map_ref(&v2, |e1, e2| *e1 / *e2);
        result.for_each(|v| assert!(v == 1, 0));
        v1.destroy();
        v2.destroy();
    }

    #[test]
    // zip_mut is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::ESMART_VECTORS_LENGTH_MISMATCH, location = Self)]
    fun smart_vector_test_zip_mut_mismatching_lengths_should_fail() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(99);
        let s = 0;
        v1.zip_mut(&mut v2, |e1, e2| s += *e1 / *e2);
        v1.destroy();
        v2.destroy();
    }

    #[test]
    // zip_map is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::ESMART_VECTORS_LENGTH_MISMATCH, location = Self)]
    fun smart_vector_test_zip_map_mismatching_lengths_should_fail() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(99);
        v1.zip_map(v2, |e1, e2| e1 / e2).destroy();
    }

    #[test]
    // zip_map_ref is an inline function so any error code will be reported at the call site.
    #[expected_failure(abort_code = V::ESMART_VECTORS_LENGTH_MISMATCH, location = Self)]
    fun smart_vector_test_zip_map_ref_mismatching_lengths_should_fail() {
        let v1 = make_smart_vector(100);
        let v2 = make_smart_vector(99);
        v1.zip_map_ref(&v2, |e1, e2| *e1 / *e2).destroy();
        v1.destroy();
        v2.destroy();
    }
}
