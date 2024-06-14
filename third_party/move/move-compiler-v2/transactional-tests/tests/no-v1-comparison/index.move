//# publish
module 0x42::test {

    struct R has key, drop { value: bool }

    const CONSTANT_VEC: vector<u8> = vector[1, 2, 3];

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun test_resource_1() acquires R {
        use 0x42::test;
        assert!((&test::R[@0x1]).value == true, 0);
    }

    fun test_resource_2() acquires R {
        let x = &mut 0x42::test::R[@0x1];
        x.value = false;
        assert!((&R[@0x1]).value == false, 1);
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

    fun test_resource_3() acquires Y {
        use 0x42::test;
        assert!((&test::Y<X<bool>>[@0x1]).field.value == true, 0);
    }

    fun test_resource_4() acquires Y {
        let addr = @0x1;
        let y = &mut 0x42::test ::Y<X<bool>> [addr];
        y.field.value = false;
        spec {
            assert Y<X<bool>>[addr].field.value == false;
        };
        assert!((&Y<X<bool>>[addr]) .field.value == false, 1);
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
        assert!((&v[0]).field.value == true, 0);
        assert!((&v[1]).field.value == false, 0);
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
        assert!((&v[0]).field.value == true, 0);
        assert!((&v[1]).field.value == false, 0);
        (&mut v[0]).field.value = false;
        (&mut v[1]).field.value = true;
        assert!((&v[0]).field.value == false, 0);
        assert!((&v[1]).field.value == true, 0);
    }

    fun test_vector_const() {
        assert!(CONSTANT_VEC[0] == 1, 0);
    }

    struct M has drop, copy {
        vec: vector<u8>,
    }

    fun test_vector_in_struct() {
        let x = M {
            vec: vector[1, 2, 3]
        };
        let v = vector[x, x];
        assert!(v[0].vec == vector[1,2,3], 0);
    }

    fun test_vector_in_struct_2() {
        let x = M {
            vec: vector[1, 2, 3]
        };
        let v = vector[x, x];
        assert!(v[0].vec[0] == 1, 0);
    }

    fun test_vector_in_struct_3() {
        let x = M {
            vec: vector[1, 2, 3]
        };
        let v = vector[x, x];
        let y = &(v[0].vec)[2];
        assert!(*y == 3, 0);
    }

}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose -- 0x42::test::test_resource_1

//# run --verbose -- 0x42::test::test_resource_2

//# run --verbose -- 0x42::test::test_vector

//# run --verbose -- 0x42::test::test_vector_borrow

//# run --verbose -- 0x42::test::test_vector_borrow_mut

//# run --verbose --signers 0x1 -- 0x42::test::init_2

//# run --verbose -- 0x42::test::test_resource_3

//# run --verbose -- 0x42::test::test_resource_4

//# run --verbose -- 0x42::test::test_vector_const

//# run --verbose -- 0x42::test::test_vector_in_struct

//# run --verbose -- 0x42::test::test_vector_in_struct_2

//# run --verbose -- 0x42::test::test_vector_in_struct_3
