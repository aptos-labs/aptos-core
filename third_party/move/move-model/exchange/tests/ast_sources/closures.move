module 0x42::closures {
    fun add(x: u64, y: u64): u64 {
        x + y
    }

    fun pick<T: copy + drop>(first: bool, a: T, b: T): T {
        if (first) a else b
    }

    inline fun twice(f: |u64| u64, x: u64): u64 {
        f(f(x))
    }

    fun leading(x: u64, y: u64): u64 {
        let f = |z| add(x, z);
        f(y)
    }

    fun trailing(x: u64, y: u64): u64 {
        let f = |z| add(z, y);
        f(x)
    }

    fun generic(first: bool, a: u64, b: u64): u64 {
        let f = |p, q| pick<u64>(first, p, q);
        f(a, b)
    }

    fun inlined(x: u64): u64 {
        twice(|z| z + 1, x)
    }
}
