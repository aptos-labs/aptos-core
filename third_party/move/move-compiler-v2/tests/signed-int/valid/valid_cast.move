module 0x42::valid_cast {
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

    fun test_cast1(x: u64): i64 {
        x as i64
    }

    fun test_cast2(x: i64): u64 {
        x as u64
    }

    fun test_cast3(x: u128): i128 {
        x as i128
    }

    fun test_cast4(x: i128): u128 {
        x as u128
    }

    fun test_cast5(a: i64, b: i128): u64 {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i64> {x: a, y: s1, z: s2};
        (s1.y as u64) + (s2.y as u64) + (s3.x as u64)
    }

    fun test_cast6(a: i64, b: i128): u128 {
        let s1 = S1 {x: 1, y: a, z: b};
        let s2 = S2 {x: s1, y: a, z: b};
        let s3 = S3<i128> {x: b, y: s1, z: s2};
        (s1.z as u128) + (s2.z as u128) + (s3.x as u128)
    }
}
