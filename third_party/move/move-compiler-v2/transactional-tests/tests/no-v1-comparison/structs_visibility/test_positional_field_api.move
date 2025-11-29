// TODO: #18199
//# publish
module 0x42::m1 {
    public struct Pair(u64, bool) has copy, drop;

    public struct Wrapper<T: drop>(T) has drop;

    public struct NestedPair<T: drop>(Pair, Wrapper<T>) has drop;

    public struct VecWrap<T: drop>(vector<T>) has drop;
}

//# publish
module 0x42::m2 {
    use 0x42::m1::{Pair, Wrapper, NestedPair, VecWrap};

    public fun test_borrow_pair_fields() {
        let p = Pair(42, true);
        let x = &p.0;
        let y = &p.1;
        assert!(*x == 42, 0);
        assert!(*y, 1);
    }

    public fun test_borrow_wrapper_field() {
        let w = Wrapper<u64>(100);
        let inner = &w.0;
        assert!(*inner == 100, 2);
    }

    public fun test_borrow_nested_pair_fields() {
        let p = Pair(1, false);
        let w = Wrapper<u64>(99);
        let np = NestedPair<u64>(p, w);

        let inner_pair_0 = &np.0.0;
        let inner_pair_1 = &np.0.1;
        let wrapped_val = &np.1.0;

        assert!(*inner_pair_0 == 1, 3);
        assert!(!*inner_pair_1, 4);
        assert!(*wrapped_val == 99, 5);
    }

    public fun test_borrow_vecwrap_element() {
        let v = VecWrap<u64>(vector[10, 20, 30]);
        let first = &v.0[0];
        let second = &v.0[1];
        assert!(*first == 10, 6);
        assert!(*second == 20, 7);
    }

    public fun test_mut_borrow_pair() {
        let p = Pair(7, false);
        let r = &mut p;
        let r0 = &mut r.0;
        *r0 = 8;
        assert!(p.0 == 8, 8);
    }

    public fun test_mut_borrow_vecwrap_element() {
        let v = VecWrap<u64>(vector[1, 2, 3]);
        let r = &mut v;
        let el = &mut r.0[1];
        *el = 10;
        assert!(v.0[1] == 10, 9);
    }

    public fun test_mut_borrow_nestedpair() {
        let p = Pair(0, true);
        let w = Wrapper<u64>(77);
        let np = NestedPair<u64>(p, w);
        let r = &mut np;
        let b1 = &mut r.0.0;
        *b1 = 123;
        let b2 = &mut r.1.0;
        *b2 = 456;
        assert!(np.0.0 == 123, 10);
        assert!(np.1.0 == 456, 11);
    }

    public fun test_mut_borrow_nestedpair_2() {
        let p = Pair(0, true);
        let w = Wrapper<u64>(77);
        let np = NestedPair<u64>(p, w);
        let NestedPair(p, w) = &mut np;
        let Pair(b1, _) = p;
        let Wrapper(b2) = w;
        *b1 = 123;
        *b2 = 456;
        assert!(np.0.0 == 123, 10);
        assert!(np.1.0 == 456, 11);
    }
}


//# run 0x42::m2::test_borrow_pair_fields

//# run 0x42::m2::test_borrow_wrapper_field

//# run 0x42::m2::test_borrow_nested_pair_fields

//# run 0x42::m2::test_borrow_vecwrap_element

//# run 0x42::m2::test_mut_borrow_pair

//# run 0x42::m2::test_mut_borrow_vecwrap_element

//# run 0x42::m2::test_mut_borrow_nestedpair

//# run 0x42::m2::test_mut_borrow_nestedpair_2
