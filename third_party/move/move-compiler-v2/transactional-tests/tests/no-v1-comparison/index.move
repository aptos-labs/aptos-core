//# publish
module 0x42::test {

    struct R has key, drop { value: bool }

    const CONSTANT_VEC: vector<u8> = vector[1, 2, 3];

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun test_resource_1() acquires R {
        use 0x42::test;
        assert!(test::R[@0x1].value == true, 0);
        R[@0x1] = R{value: false};
        assert!(test::R[@0x1].value == false, 0);
    }

    fun test_resource_2() acquires R {
        let x = &mut 0x42::test::R[@0x1];
        x.value = false;
        assert!(R[@0x1].value == false, 1);
    }

    struct X<M> has copy, drop, store {
        value: M
    }
    struct Y<T> has key, drop, copy {
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
        assert!(test::Y<X<bool>>[@0x1].field.value == true, 0);
    }

    fun test_resource_freeze() acquires Y {
        use 0x42::test;
        let y: &Y<X<bool>> = &mut test::Y<X<bool>>[@0x1];
        assert!(y.field.value == true, 0);
    }

    fun test_resource_4() acquires Y {
        let addr = @0x1;
        let y = &mut 0x42::test ::Y<X<bool>> [addr];
        y.field.value = false;
        spec {
            assert Y<X<bool>>[addr].field.value == false;
        };
        assert!(Y<X<bool>>[addr].field.value == false, 1);
    }

    fun test_resource_5() acquires Y {
        let addr = @0x1;
        0x42::test ::Y<X<bool>> [addr].field.value = false;
        spec {
            assert Y<X<bool>>[addr].field.value == false;
        };
        let y_resource = Y<X<bool>>[addr];
        assert!(y_resource.field.value == false, 1);
    }

    fun test_vector() {
        let x = X {
            value: 2
        };
        let v = vector[x, x];
        assert!(v[0].value == 2, 0);
        v[0].value = 3;
        assert!(v[0].value == 3, 0);
    }

    fun test_two_dimension() {
        let x = vector[vector[1, 2], vector[3, 4]];
        assert!(x[0][0] == 1, 0);
        x[0] = vector[2, 3, 4];
        assert!(x[0][0] == 2, 0);
        x[0][1] = 4;
        assert!(x[0][1] == 4, 0);
        let x = vector[vector[1, 2], vector[3, 4]];
        let y = 0;
        assert!(x[{y = y + 1; y}][{y = y - 1; y}] == 3, 0);
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

    fun foo(v: &Y<X<bool>>) {
        assert!(v.field.value == true, 0);
    }

    fun test_vector_borrow_freeze() {
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
        let v_y1: &Y<X<bool>> = &mut v[0];
        assert!(v_y1.field.value == true, 0);
        assert!(v[0].field.value == true, 0);
        assert!(v[1].field.value == false, 0);
        foo(&mut v[0]);
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
        assert!(v[1].vec[0] == 1, 0);
    }

    fun init_3(s: &signer) {
        let y = Y {
            field: vector[(1 as u8),2,3]
        };
        move_to(s, y);
    }

    fun test_resource_with_vector() acquires Y {
        assert!(Y<vector<u8>>[@0x2].field[0] == 1, 0);
    }

    fun bubble_sort(v: vector<u64>) {
        use std::vector;
        let n = vector::length(&v);
        let i = 0;

        while (i < n) {
           let j = 0;
           while (j < n - i - 1) {
               if (v[j] > v[j + 1]) {
                   let t = v[j];
                   v[j] = v[j + 1];
                   v[j + 1] = t;
               };
           j = j + 1;
           };
           i = i + 1;
        };
        assert!(v[0] == 1, 0);
    }

    fun call_sort() {
        let v = vector[3, 1, 2];
        bubble_sort(v);
    }

    fun test_index_then_field_select_1() {
        let x1 = X {
            value: true
        };
        let v = vector[x1];
        let p = &mut v[0].value;
        *p = false;
        assert!(v[0].value == false, 0);
    }

    fun test_index_then_field_select_2() {
        let x1 = X {
            value: true
        };
        let v = &mut vector[x1];
        let p = &mut v[0].value;
        *p = false;
        assert!(v[0].value == false, 0);
    }

    fun test_index_then_field_select_3() {
        let x1 = X {
            value: true
        };
        let v = &vector[x1];
        assert!(v[0].value == true, 0);
    }

    fun inc_vec_new(x: &mut vector<u256>, index: u64) {
        x[index] = x[index] + 1;
    }

    fun inc_vec_new_test() {
        let x = vector[0];
        x[0] = x[0] + 1;
        assert!(x[0] == 1, 0);
        let y = &mut x;
        inc_vec_new(y, 0);
        assert!(y[0] == 2, 0);
    }

}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose -- 0x42::test::test_resource_1

//# run --verbose -- 0x42::test::test_resource_2

//# run --verbose -- 0x42::test::test_vector

//# run --verbose -- 0x42::test::test_two_dimension

//# run --verbose -- 0x42::test::test_vector_borrow

//# run --verbose -- 0x42::test::test_vector_borrow_freeze

//# run --verbose -- 0x42::test::test_vector_borrow_mut

//# run --verbose --signers 0x1 -- 0x42::test::init_2

//# run --verbose -- 0x42::test::test_resource_3

//# run --verbose -- 0x42::test::test_resource_freeze

//# run --verbose -- 0x42::test::test_resource_4

//# run --verbose -- 0x42::test::test_resource_5

//# run --verbose -- 0x42::test::test_vector_const

//# run --verbose -- 0x42::test::test_vector_in_struct

//# run --verbose -- 0x42::test::test_vector_in_struct_2

//# run --verbose -- 0x42::test::test_vector_in_struct_3

//# run --verbose --signers 0x2 -- 0x42::test::init_3

//# run --verbose -- 0x42::test::test_resource_with_vector

//# run --verbose -- 0x42::test::call_sort

//# run --verbose -- 0x42::test::test_index_then_field_select_1

//# run --verbose -- 0x42::test::test_index_then_field_select_2

//# run --verbose -- 0x42::test::test_index_then_field_select_3

//# run --verbose -- 0x42::test::inc_vec_new_test
