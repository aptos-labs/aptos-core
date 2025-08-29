module 0x42::valid_logic {
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

    fun test_cmp1(x: i64): bool {
        x == x && x >= x && x<= x && x > x && x < x && &x == &x && &x >= &x
    }

    fun test_cmp2(x: i128): bool {
        x == x && x >= x && x<= x && x > x && x < x && &x == &x && &x >= &x
    }

    fun test_cmp3(s1: S1, s2: S2, s3: S3<i64>): bool {
        s1.y == s2.y && s1.y <= s3.x && s2.y >= s3.x && s3.x > s1.y && s3.x < s2.y && &s1.y == &s2.y && &s1.y == &s2.y
    }

    fun test_cmp4(s1: S1, s2: S2, s3: S3<i128>): bool {
        s1.z == s2.z && s1.z <= s3.x && s2.z >= s3.x && s3.x > s1.z && s3.x < s2.z && &s1.z == &s2.z && &s1.z == &s2.z
    }

    fun test_mix1(x: i64, y: i64): bool {
        x + y == y + x
    }

    fun test_mix2(x: i128, y: i128): bool {
        x + 2*y <= x + 3*y
    }

    fun test_mix3(x: i64, y: i64): bool {
        x - y == y - x
    }

    fun test_mix4(x: i128, y: i128): bool {
        x - 2*y > x -3*y
    }

    fun test_mix5(x: i64, y: i64): bool {
        x * y == y * x
    }

    fun test_mix6(x: i128, y: i128): bool {
        x * 2*y > x * 3*y
    }

    fun test_mix7(x: i64, y: i64): bool {
        x / y == y / x
    }

    fun test_mix8(x: i128, y: i128): bool {
        x / 2*y > x / 3*y
    }

    fun test_mix9(x: i64, y: i64): bool {
        x % y == y % x
    }

    fun test_mix10(x: i128, y: i128): bool {
        x % 2*y > x % 3*y
    }
}
