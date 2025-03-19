module 0xcafe::m {

    /// A higher order function on ints
    fun map(x: u64, f: |u64|u64 has drop): u64 {
        f(x)
    }

    /// Tests basic usage, without name overlap
    fun no_name_clash(x: u64, c: u64): u64 {
        map(x, |y| y + c)
    }

    /// Basic usage in the presence of name clask
    fun with_name_clash1(x: u64, c: u64): u64 {
        map(x, |x| x + c)
    }

    /// More clashes
    fun with_name_clash2(x: u64, c: u64): u64 {
        map(x, |x| {
            let x = c + 1;
            x
        } + x)
    }
}
