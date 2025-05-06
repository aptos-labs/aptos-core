module 0xc0ffee::m {
    fun make_bad_func_1(): |u64|u64 has copy + drop {
        |x: u64| {
            return (|y| x + y)
        }
    }

    fun make_bad_func_2(): |u64, u64|(|u64|u64) has copy + drop {
        return (|x, y| {
            return (|| x * y)
        })
    }

    fun make_bad_func_3(): |u64, u64|(||u64) has copy + drop {
        return (|x, y| {
            return (|| (x, y))
        })
    }
}
