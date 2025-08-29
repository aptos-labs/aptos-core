module 0x42::valid_fv {
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

    fun test_64(x: i64): i64 {
        x + x
    }

    fun test_128(x: i128): i128 {
        x + x
    }

    fun test1(fv1: |i64| i64 has copy+drop, x: i64) : i64 {
        let fv2: |i64| i64 has copy+drop = |x| x * x; // function value involving i64
        fv1(x)
    }

    fun test2(fv1: |i128| i128 has copy+drop, x: i128) : i128 { // function value involving i128
        let fv2: |i128| i128 has copy+drop = |x| x * x;
        fv1(x)
    }

    fun test3(fv1: |(|i64|i64)| i64 has copy+drop, x: i64) : i64 { // function value involving nested function value with i64
        fv1(test_64)
    }

    fun test4(fv1: |(|i128|i128)| i128 has copy+drop, x: i128) : i128 { // function value involving nested function value with i128
        fv1(test_128)
    }

    fun test5(fv: |S1| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: a, z: b};
        fv(s1)
    }

    fun test6(fv: |S2| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        fv(s2)
    }

    fun test7(fv: |S3<i64>| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: a, y: s1, z: s2};
        fv(s3)
    }

    fun test8(fv: |S3<i128>| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: b, y: s1, z: s2};
        fv(s3)
    }

    fun test9(fv: |E1| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let e = E1::V1{s: s1};
        fv(e)
    }

    fun test10(fv: |E2| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let e = E2::V2{s: s2};
        fv(e)
    }

    fun test11(fv: |E3<i64>| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i64> {x: -1, y: s1, z: s2};
        let e = E3::V3{s: s3};
        fv(e)
    }

    fun test12(fv: |E3<i128>| i64 has copy+drop, a: i64, b: i128) : i64 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        let s2 = S2 {x: s1, y: -1, z: -2};
        let s3 = S3<i128> {x: -1, y: s1, z: s2};
        let e = E3::V3{s: s3};
        fv(e)
    }
}
