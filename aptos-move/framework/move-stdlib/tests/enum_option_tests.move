#[test_only]
module std::enum_option_tests {
    use std::enum_option;
    use std::vector;

    #[test]
    fun option_none_is_none() {
        let none = enum_option::none<u64>();
        assert!(enum_option::is_none(&none), 0);
        assert!(!enum_option::is_some(&none), 1);
    }

    #[test]
    fun option_some_is_some() {
        let some = enum_option::some(5);
        assert!(!enum_option::is_none(&some), 0);
        assert!(enum_option::is_some(&some), 1);
    }

    #[test]
    fun option_contains() {
        let none = enum_option::none<u64>();
        let some = enum_option::some(5);
        let some_other = enum_option::some(6);
        assert!(enum_option::contains(&some, &5), 0);
        assert!(enum_option::contains(&some_other, &6), 1);
        assert!(!enum_option::contains(&none, &5), 2);
        assert!(!enum_option::contains(&some_other, &5), 3);
    }

    #[test]
    fun option_borrow_some() {
        let some = enum_option::some(5);
        let some_other = enum_option::some(6);
        assert!(*enum_option::borrow(&some) == 5, 3);
        assert!(*enum_option::borrow(&some_other) == 6, 4);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_NOT_SET)]
    fun option_borrow_none() {
        enum_option::borrow(&enum_option::none<u64>());
    }

    #[test]
    fun borrow_mut_some() {
        let some = enum_option::some(1);
        let ref = enum_option::borrow_mut(&mut some);
        *ref = 10;
        assert!(*enum_option::borrow(&some) == 10, 0);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_NOT_SET)]
    fun borrow_mut_none() {
        enum_option::borrow_mut(&mut enum_option::none<u64>());
    }

    #[test]
    fun borrow_with_default() {
        let none = enum_option::none<u64>();
        let some = enum_option::some(5);
        assert!(*enum_option::borrow_with_default(&some, &7) == 5, 0);
        assert!(*enum_option::borrow_with_default(&none, &7) == 7, 1);
    }

    #[test]
    fun get_with_default() {
        let none = enum_option::none<u64>();
        let some = enum_option::some(5);
        assert!(enum_option::get_with_default(&some, 7) == 5, 0);
        assert!(enum_option::get_with_default(&none, 7) == 7, 1);
    }

    #[test]
    fun extract_some() {
        let opt = enum_option::some(1);
        assert!(enum_option::extract(&mut opt) == 1, 0);
        assert!(enum_option::is_none(&opt), 1);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_NOT_SET)]
    fun extract_none() {
        enum_option::extract(&mut enum_option::none<u64>());
    }

    #[test]
    fun swap_some() {
        let some = enum_option::some(5);
        assert!(enum_option::swap(&mut some, 1) == 5, 0);
        assert!(*enum_option::borrow(&some) == 1, 1);
    }

    #[test]
    fun swap_or_fill_some() {
        let some = enum_option::some(5);
        assert!(enum_option::swap_or_fill(&mut some, 1) == enum_option::some(5), 0);
        assert!(*enum_option::borrow(&some) == 1, 1);
    }

    #[test]
    fun swap_or_fill_none() {
        let none = enum_option::none();
        assert!(enum_option::swap_or_fill(&mut none, 1) == enum_option::none(), 0);
        assert!(*enum_option::borrow(&none) == 1, 1);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_NOT_SET)]
    fun swap_none() {
        enum_option::swap(&mut enum_option::none<u64>(), 1);
    }

    #[test]
    fun fill_none() {
        let none = enum_option::none<u64>();
        enum_option::fill(&mut none, 3);
        assert!(enum_option::is_some(&none), 0);
        assert!(*enum_option::borrow(&none) == 3, 1);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_IS_SET)]
    fun fill_some() {
        enum_option::fill(&mut enum_option::some(3), 0);
    }

    #[test]
    fun destroy_with_default() {
        assert!(enum_option::destroy_with_default(enum_option::none<u64>(), 4) == 4, 0);
        assert!(enum_option::destroy_with_default(enum_option::some(4), 5) == 4, 1);
    }

    #[test]
    fun destroy_some() {
        assert!(enum_option::destroy_some(enum_option::some(4)) == 4, 0);
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_NOT_SET)]
    fun destroy_some_none() {
        enum_option::destroy_some(enum_option::none<u64>());
    }

    #[test]
    fun destroy_none() {
        enum_option::destroy_none(enum_option::none<u64>());
    }

    #[test]
    #[expected_failure(abort_code = enum_option::EOPTION_IS_SET)]
    fun destroy_none_some() {
        enum_option::destroy_none(enum_option::some<u64>(0));
    }

    #[test]
    fun into_vec_some() {
        let v = enum_option::to_vec(enum_option::some<u64>(0));
        assert!(vector::length(&v) == 1, 0);
        let x = vector::pop_back(&mut v);
        assert!(x == 0, 1);
    }

    #[test]
    fun into_vec_none() {
        let v: vector<u64> = enum_option::to_vec(enum_option::none());
        assert!(vector::is_empty(&v), 0);
    }

    #[test]
    fun test_for_each() {
        let r = 0;
        enum_option::for_each(enum_option::some(1), |x| r = x);
        assert!(r == 1, 0);
        r = 0;
        enum_option::for_each(enum_option::none<u64>(), |x| r = x);
        assert!(r == 0, 1);
    }

    #[test]
    fun test_for_each_ref() {
        let r = 0;
        enum_option::for_each_ref(&enum_option::some(1), |x| r = *x);
        assert!(r == 1, 0);
        r = 0;
        enum_option::for_each_ref(&enum_option::none<u64>(), |x| r = *x);
        assert!(r == 0, 1);
    }

    #[test]
    fun test_for_each_mut() {
        let o = enum_option::some(0);
        enum_option::for_each_mut(&mut o, |x| *x = 1);
        assert!(o == enum_option::some(1), 0);
    }

    #[test]
    fun test_fold() {
        let r = enum_option::fold(enum_option::some(1), 1, |a, b| a + b);
        assert!(r == 2, 0);
        let r = enum_option::fold(enum_option::none<u64>(), 1, |a, b| a + b);
        assert!(r == 1, 0);
    }

    #[test]
    fun test_map() {
        let x = enum_option::map(enum_option::some(1), |e| e + 1);
        assert!(enum_option::extract(&mut x) == 2, 0);
    }

    #[test]
    fun test_map_ref() {
        let x = enum_option::map_ref(&enum_option::some(1), |e| *e + 1);
        assert!(enum_option::extract(&mut x) == 2, 0);
    }

    #[test]
    fun test_filter() {
        let x = enum_option::filter(enum_option::some(1), |e| *e != 1);
        assert!(enum_option::is_none(&x), 0);
    }

    #[test]
    fun test_any() {
        let r = enum_option::any(&enum_option::some(1), |e| *e == 1);
        assert!(r, 0);
    }

    #[test]
    fun test_bcs_equivalent() {
        assert!(std::bcs::to_bytes(&std::option::some(5)) == std::bcs::to_bytes(&std::enum_option::some(5)), 0);
        assert!(std::bcs::to_bytes(&std::option::none<u64>()) == std::bcs::to_bytes(&std::enum_option::none<u64>()), 1);
    }
}
