//# publish
module 0x42::vector {
    fun create(): vector<u64> {
        vector[1, 2, 3]
    }

    public fun test_fold() {
        use std::vector;
        let v = vector[1];
        let accu = vector::fold(v, 0, |_, _| 0 );
        assert!(accu == 0 , 0)
    }

    public fun test_remove()  {
        use std::vector;
        let v = create();
        let e = remove(&mut v, 1);
        assert!(e == 2, 0);
        assert!(vector::length(&v) == 2, 0);
        assert!(*vector::borrow(&v, 0) == 1, 0);
        assert!(*vector::borrow(&v, 1) == 3, 0);
    }

    public fun remove<Element>(v: &mut vector<Element>, i: u64): Element {
        use std::vector;
        let len = vector::length(v);
        if (i >= len) abort 1;

        len = len - 1;
        while (i < len) vector::swap(v, i, { i = i + 1; i });
        vector::pop_back(v)
    }
}

//# run 0x42::vector::test_fold

//# run 0x42::vector::test_remove
