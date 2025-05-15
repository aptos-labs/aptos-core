 module 0x0fff::cmp {
    public fun test1<T: drop>(x: T, y: T): bool {
        x < y
        // => cmp::compare(&x, &y).is_lt()
    }
}
