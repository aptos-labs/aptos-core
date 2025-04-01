#[test_only]
module std::mem_tests {
    use std::mem::{swap, replace};

    #[test]
    fun test_swap_ints() {
        let a = 1;
        let b = 2;
        let v = vector[3, 4, 5, 6];

        swap(&mut a, &mut b);
        assert!(a == 2, 0);
        assert!(b == 1, 1);

        swap(&mut a, &mut v[0]);
        assert!(a == 3, 0);
        assert!(v[0] == 2, 1);

        swap(&mut v[2], &mut a);
        assert!(a == 5, 0);
        assert!(v[2] == 3, 1);
    }

    #[test]
    fun test_replace_ints() {
        let a = 1;
        let b = 2;

        assert!(replace(&mut a, b) == 1, 0);
        assert!(a == 2, 1);
    }

    #[test_only]
    struct SomeStruct has drop, key {
        f: u64,
        v: vector<u64>,
    }

    #[test]
    fun test_swap_struct() {
        let a = 1;
        let v = vector[20, 21];
        let s1 = SomeStruct { f: 2, v: vector[3, 4] };
        let s2 = SomeStruct { f: 5, v: vector[6, 7] };
        let vs = vector[SomeStruct { f: 8, v: vector[9, 10] }, SomeStruct { f: 11, v: vector[12, 13] }];

        swap(&mut s1, &mut s2);
        assert!(&s1 == &SomeStruct { f: 5, v: vector[6, 7] }, 0);
        assert!(&s2 == &SomeStruct { f: 2, v: vector[3, 4] }, 1);

        swap(&mut s1.f, &mut a);
        assert!(s1.f == 1, 2);
        assert!(a == 5, 3);

        swap(&mut s1.f, &mut s1.v[0]);
        assert!(s1.f == 6, 4);
        assert!(s1.v[0] == 1, 5);

        swap(&mut s2, &mut vs[0]);
        assert!(&s2 == &SomeStruct { f: 8, v: vector[9, 10] }, 6);
        assert!(&vs[0] == &SomeStruct { f: 2, v: vector[3, 4] }, 7);

        swap(&mut s1.f, &mut v[0]);
        assert!(s1.f == 20, 8);
        assert!(v[0] == 6, 9);
    }

    #[test(creator = @0xcafe)]
    fun test_swap_resource(creator: &signer) acquires SomeStruct {
        use std::signer;
        {
            move_to(creator, SomeStruct { f: 5, v: vector[6, 7] });
        };

        {
            let value = &mut SomeStruct[signer::address_of(creator)];
            let s1 = SomeStruct { f: 2, v: vector[3, 4] };
            let vs = vector[SomeStruct { f: 8, v: vector[9, 10] }, SomeStruct { f: 11, v: vector[12, 13] }];

            swap(&mut s1, value);
            assert!(&s1 == &SomeStruct { f: 5, v: vector[6, 7] }, 0);
            assert!(value == &SomeStruct { f: 2, v: vector[3, 4] }, 1);

            swap(value, &mut vs[0]);
            assert!(value == &SomeStruct { f: 8, v: vector[9, 10] }, 2);
            assert!(&vs[0] == &SomeStruct { f: 2, v: vector[3, 4] }, 3);

            let v_ref = &mut value.v;
            let other_v = vector[11, 12];
            swap(v_ref, &mut other_v);

            assert!(v_ref == &vector[11, 12], 4);
            assert!(&other_v == &vector[9, 10], 5);
        }
    }
}
