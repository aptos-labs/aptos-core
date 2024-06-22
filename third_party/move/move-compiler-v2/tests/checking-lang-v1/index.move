module 0x42::test {

    struct R has key, drop { value: bool }

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun test_resource_1() acquires R {
        use 0x42::test;
        assert!((test::R[@0x1]).value == true, 0);
    }

    fun test_resource_2() acquires R {
        0x42::test::R[@0x1].value = false;
        assert!(R[@0x1].value == false, 1);
    }

    struct X<M> has copy, drop, store {
        value: M
    }
    struct Y<T> has key, drop {
        field: T
    }

    fun init_2(s: &signer) {
        let x = X {
            value: true
        };
        let y = Y {
            field: x
        };
        move_to(s, y);
    }

    fun test_resource_3() {
        use 0x42::test;
        assert!(test::Y<X<bool>>[@0x1].field.value == true, 0);
    }

    fun test_resource_4() {
        let addr = @0x1;
        let y = &mut 0x42::test ::Y<X<bool>> [addr];
        y.field.value = false;
        assert!(Y<X<bool>>[addr].field.value == false, 1);
    }

    fun test_vector() {
        let x = X {
            value: 2
        };
        let v = vector[x, x];
        assert!(v[0].value == 2, 0);
    }

    fun test_vector_borrow() {
        let x1 = X {
            value: true
        };
        let x2 = X {
            value: false
        };
        let y1 = Y {
            field: x1
        };
        let y2 = Y {
            field: x2
        };
        let v = vector[y1, y2];
        assert!(v[0].field.value == true, 0);
        assert!(v[1].field.value == false, 0);
    }

    fun test_vector_borrow_mut() {
        let x1 = X {
            value: true
        };
        let x2 = X {
            value: false
        };
        let y1 = Y {
            field: x1
        };
        let y2 = Y {
            field: x2
        };
        let v = vector[y1, y2];
        assert!(v[0].field.value == true, 0);
        assert!(v[1].field.value == false, 0);
        v[0].field.value = false;
        v[1].field.value = true;
        assert!(v[0].field.value == false, 0);
        assert!(v[1].field.value == true, 0);
    }



}
