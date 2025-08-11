module 0xcafe::m {

    /// Test struct
    struct S<T> {
        x: T
    }

    /// A higher order function on `S`
    fun consume<T>(s: S<T>, x: T, f: |S<T>, T|T): T {
        f(s, x)
    }

    /// Lambda with pattern
    fun pattern(s: S<u64>, x: u64): u64 {
        consume(s, x, |S{x}, _y| { let y = x; x + y})
    }

}
