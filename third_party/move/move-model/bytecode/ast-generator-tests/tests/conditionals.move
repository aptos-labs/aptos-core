module 0x815::m {

    fun if_else_1(c: bool): u8 {
        if (c) 1 else 2
    }

    fun if_else_2(c: bool, d: bool): u8 {
        if (c) {
            if (d) {
                1
            } else {
                2
            }
        } else {
            3
        }
    }


    fun if_1(c: bool): u8 {
        let result = 0;
        if (c) {
            result = 1;
        };
        result
    }

    fun if_else_3(c: bool): u64 {
        let r = if (c) 1 else 2;
        r
    }

    fun if_else_with_shard_exp(x: u64): u64 {
        let y = x + x;
        let z = y * y;
        if (z > 0) z + 1 else z - 1
    }
}
