module 0xcafe::m {

    /// A higher order function on ints
    fun map1(x: u64, f: |u64|u64): u64 {
        f(x)
    }

    /// Another higher order function on ints
    fun map2(x: u8, f: |u8|u8): u8 {
        f(x)
    }

    /// A tests which nests things
    fun nested(x: u64, c: u64): u64 {
        map1(x, |y| (map2((y - c as u8), |y| y + (c as u8)) as u64))
    }
}
