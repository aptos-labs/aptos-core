#[test_only]
module std::mem_tests {
    use std::vector;
    use std::mem::{swap, replace};

    #[test]
    fun test_swap_ints() {
        let a = 1;
        let b = 2;
        let v = vector[3, 4, 5, 6];

        swap(&mut a, &mut b);
        assert!(a == 2, 0);
        assert!(b == 1, 1);

        swap(&mut a, vector::borrow_mut(&mut v, 0));
        assert!(a == 3, 0);
        assert!(vector::borrow(&v, 0) == &2, 1);

        swap(vector::borrow_mut(&mut v, 2), &mut a);
        assert!(a == 5, 0);
        assert!(vector::borrow(&v, 2) == &3, 1);
    }


    #[test]
    fun test_replace_ints() {
        let a = 1;
        let b = 2;

        assert!(replace(&mut a, b) == 1, 0);
        assert!(a == 2, 1);
    }

    #[test_only]
    struct SomeStruct has drop {
        f: u64,
        v: vector<u64>,
    }

    #[test]
    fun test_swap_struct() {
        let a = 1;
        let s1 = SomeStruct { f: 2, v: vector[3, 4]};
        let s2 = SomeStruct { f: 5, v: vector[6, 7]};
        let vs = vector[SomeStruct { f: 8, v: vector[9, 10]}, SomeStruct { f: 11, v: vector[12, 13]}];


        swap(&mut s1, &mut s2);
        assert!(&s1 == &SomeStruct { f: 5, v: vector[6, 7]}, 0);
        assert!(&s2 == &SomeStruct { f: 2, v: vector[3, 4]}, 1);

        swap(&mut s1.f, &mut a);
        assert!(s1.f == 1, 2);
        assert!(a == 5, 3);

        swap(&mut s1.f, vector::borrow_mut(&mut s1.v, 0));
        assert!(s1.f == 6, 4);
        assert!(vector::borrow(&s1.v, 0) == &1, 5);

        swap(&mut s2, vector::borrow_mut(&mut vs, 0));
        assert!(&s2 == &SomeStruct { f: 8, v: vector[9, 10]}, 6);
        assert!(vector::borrow(&vs, 0) == &SomeStruct { f: 2, v: vector[3, 4]}, 7);
    }
}
