module 0x42::valid_vector {
    use std::vector;

    struct S1  has copy, drop { x: u64, y: i64, z: i128 }

    struct S2 has copy, drop { x: S1, y: i64, z: i128  }

    struct S3<T>  has copy, drop { x: T, y: S1, z: S2 }

    enum E1 has copy, drop {
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<i64>},
    }

    enum E2 has copy, drop {
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<i128>},
    }

    enum E3<T> has copy, drop {
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<T>},
    }

    fun test1(a: i64): vector<i64> {
        let v = vector::empty<i64>(); // vector with i64
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, 1);
        vector::push_back(&mut v, -1);
        v
    }

    fun test2(a: i128): vector<i128> { // vector with i128
        let v = vector::empty<i128>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, 1);
        vector::push_back(&mut v, -1);
        v
    }

    fun test3(a: i64, b: i128): vector<S1> {
        let s1 = S1 {x: 1, y: a, z: b};
        let s11 = S1 {x: 1, y: -1, z: -2};
        let v = vector::empty<S1>(); // vector with struct involving i64 and i128
        vector::push_back(&mut v, s1);
        vector::push_back(&mut v, s11);
        v
    }

    fun test4(a: i64, b: i128): vector<S2> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s21 = S2 {x: s1, y: -1, z: -2};
        let v = vector::empty<S2>(); // vector with struct involving i64 and i128 and nested struct
        vector::push_back(&mut v, s2);
        vector::push_back(&mut v, s21);
        v
    }

    fun test5(a: i64, b: i128): vector<S3<i64>> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: a, y: s1, z: s2};
        let v = vector::empty<S3<i64>>(); // vector with struct involving i64, i128, nested struct, and generic types
        vector::push_back(&mut v, s3);
        vector::push_back(&mut v, s3);
        v
    }

    fun test6(a: i64, b: i128): vector<S3<i128>> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: b, y: s1, z: s2};
        let v = vector::empty<S3<i128>>(); // vector with struct involving i64, i128, nested struct, and generic types
        vector::push_back(&mut v, s3);
        vector::push_back(&mut v, s3);
        v
    }

    fun test7(a: i64, b: i128): vector<E1> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: -1, y: s1, z: s2};
        let e1 = E1::V1{s: s1};
        let e2 = E1::V2{s: s2};
        let e3 = E1::V3{s: s3};
        let v = vector::empty<E1>(); // vector with enums involving i64, i128, and nested struct,
        vector::push_back(&mut v, e1);
        vector::push_back(&mut v, e2);
        vector::push_back(&mut v, e3);
        v
    }

    fun test8(a: i64, b: i128): vector<E2> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: -1, y: s1, z: s2};
        let e1 = E2::V1{s: s1};
        let e2 = E2::V2{s: s2};
        let e3 = E2::V3{s: s3};
        let v = vector::empty<E2>(); // vector with enums involving i64, i128, and nested struct,
        vector::push_back(&mut v, e1);
        vector::push_back(&mut v, e2);
        vector::push_back(&mut v, e3);
        v
    }

    fun test9(a: i64, b: i128): vector<E3<i64>> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: -1, y: s1, z: s2};
        let e1 = E3::V1{s: s1};
        let e2 = E3::V2{s: s2};
        let e3 = E3::V3{s: s3};
        let v = vector::empty<E3<i64>>(); // vector with enums involving i64, i128, nested struct, and generic types
        vector::push_back(&mut v, e1);
        vector::push_back(&mut v, e2);
        vector::push_back(&mut v, e3);
        v
    }

    fun test10(a: i64, b: i128): vector<E3<i128>> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: -1, y: s1, z: s2};
        let e1 = E3::V1{s: s1};
        let e2 = E3::V2{s: s2};
        let e3 = E3::V3{s: s3};
        let v = vector::empty<E3<i128>>(); // vector with enums involving i64, i128, and nested struct, and generic types
        vector::push_back(&mut v, e1);
        vector::push_back(&mut v, e2);
        vector::push_back(&mut v, e3);
        v
    }
}
