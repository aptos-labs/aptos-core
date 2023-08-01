module 0x42::operators {
    fun arithm(x: u64, y: u64): u64 {
        x + y / (x - y) * y % x
    }

    fun bits(x: u64, y: u8): u64 {
        x << y & x
    }

    fun bools(x: bool, y: bool): bool {
        x && y || x && !y || !x && y || !x && !y
    }

    fun equality<T>(x: T, y: T): bool {
        x == y
    }

    fun inequality<T>(x: T, y: T): bool {
        x != y
    }

    fun order(x: u64, y: u64): bool {
        x < y && x <= y && !(x > y) && !(x >= y)
    }
}
