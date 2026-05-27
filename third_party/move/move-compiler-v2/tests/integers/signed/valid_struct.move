module 0x42::valid_struct {

    struct S1  has copy, drop { x: u64, y: i64, z: i128 } // struct with i64 and i128 fields

    struct S2 has copy, drop { x: S1, y: i64, z: i128  } // struct with i64 and i128 fields, as well as nested struct involving i64 and i128

    struct S3<T>  has copy, drop { x: T, y: S1, z: S2 } // struct with two layers of nesting, and generic types

    enum E1 has copy, drop { // enum with i64 and nested struct
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<i64>},
    }

    enum E2 has copy, drop { // enum with i128 and nested struct
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<i128>},
    }

    enum E3<T> has copy, drop { // enum with nested struct, and generic types
        V1 {s: S1},
        V2 {s: S2},
        V3 {s: S3<T>},
    }

    fun test1(a: i64, b: i128): S3<i64> {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: a, y: s1, z: s2};
        s3
    }

    fun test2(): S3<i64> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i64> {x: -1, y: s1, z: s2};
        s3
    }

    fun test3(a: i64, b: i128): S3<i128> {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: b, y: s1, z: s2};
        s3
    }

    fun test4(): S3<i128> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i128> {x: -1, y: s1, z: s2};
        s3
    }

    fun test5(): E1 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let e = E1::V1{s: s1};
        e
    }

    fun test6(): E2 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let e = E2::V2{s: s2};
        e
    }

    fun test7(): E3<i64> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i64> {x: -1, y: s1, z: s2};
        let e = E3::V3{s: s3};
        e
    }

    fun test8(): E3<i128> {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i128> {x: -1, y: s1, z: s2};
        let e = E3::V3{s: s3};
        e
    }
}
