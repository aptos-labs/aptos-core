module 0x42::valid_control_flow {
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

    fun test1(x: i64, y: i64) : i64 {
        if (x > y || x == y) {
            x
        } else {
            y
        }
    }

    fun test2(x: i128, y: i128) : i128 {
        if (x > y || x == y) {
            x
        } else {
            y
        }
    }

    fun test3(x: i64, y: i64) : i64 {
        while (! (x == 0)) {
            let y = x + 1;
            let z = x - 1;
            let res =
            if (y < z) { y }
            else if (z < y) { z }
            else break;
            x = x * 2;
        };
        x
    }

    fun test4(x: i128, y: i128) : i128 {
        while (! (x == 0)) {
            let y = x + 1;
            let z = x - 1;
            let res =
            if (y < z) { y }
            else if (z < y) { z }
            else break;
            x = x * 2;
        };
        x
    }
}
