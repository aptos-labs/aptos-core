module 0x42::test {
    struct ListGood<T> {
        head: T,
        tail: ListGood<T>,
    }

    struct ListBad<T, U> {
        head: T,
        tail: ListBad<T, U>,
    }

    struct BadFields<T, U> {
        f: (T, bool),
        g: &T,
    }
}
