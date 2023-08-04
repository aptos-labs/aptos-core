module 0x42::if_else {

    fun if_else(cond: bool, x: u64): u64 {
        if (cond) x + 1 else x - 1
    }

    fun if_else_nested(cond: bool, x: u64): u64 {
        if ((if (cond) x + 1 else x - 1) > 10) x * 2 else x / 2
    }
}
