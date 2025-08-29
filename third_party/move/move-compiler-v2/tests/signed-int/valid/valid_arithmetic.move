module 0x42::valid_arithmetic {
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

    fun test_add1(x: i64): i64 {
        x + x
    }

    fun test_add2(x: i128): i128 {
        x + x
    }

    fun test_add3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        s1.y + s2.y + s3.x
    }

    fun test_add4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        s1.z + s2.z + s3.x
    }

    fun test_sub1(x: i64, y: i64): i64 {
        x - y
    }

    fun test_sub2(x: i128, y: i128): i128 {
        x - y
    }

    fun test_sub3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        s1.y - s2.y - s3.x
    }

    fun test_sub4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        s1.z - s2.z - s3.x
    }

    fun test_mul1(x: i64, y: i64): i64 {
        x * y
    }

    fun test_mul2(x: i128, y: i128): i128 {
        x * y
    }

    fun test_mul3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        s1.y * s2.y * s3.x
    }

    fun test_mul4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        s1.z * s2.z * s3.x
    }

    fun test_div1(x: i64, y: i64): i64 {
        x / y
    }

    fun test_div2(x: i128, y: i128): i128 {
        x / y
    }

    fun test_div3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        s1.y / s2.y / s3.x
    }

    fun test_div4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        s1.z / s2.z / s3.x
    }

    fun test_mod1(x: i64, y: i64): i64 {
        x % y
    }

    fun test_mod2(x: i128, y: i128): i128 {
        x % y
    }

    fun test_mod3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        s1.y % s2.y % s3.x
    }

    fun test_mod4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        s1.z % s2.z % s3.x
    }

    fun test_mix1(x: i64, y: i64, z: i64): i64 {
        ((x + y) - z) * x / y % z
    }

    fun test_mix2(x: i128, y: i128, z: i128): i128 {
        ((x + y) - z) * x / y % z
    }

    fun test_mix3(s1: S1, s2: S2, s3: S3<i64>): i64 {
        ((s1.y + s2.y) - s3.x) * s1.y / s2.y % s3.x
    }

    fun test_mix4(s1: S1, s2: S2, s3: S3<i128>): i128 {
        ((s1.z + s2.z) - s3.x) * s1.z / s2.z % s3.x
    }

    fun test_neg1(x: i64, y: i64): i64 {
        -x + y - -x * -y / -x % -y
    }

    fun test_neg2(x: i128, y: i128): i128 {
        -x + y - -x * -y / -x % -y
    }
}
