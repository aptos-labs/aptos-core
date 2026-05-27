module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun two(): u64 {
        2
    }

    fun sum_codes(): u64 {
        code(true) + code(false)
    }

    public fun code(is_sell: bool): u64 {
        if (is_sell) {
            one()
        } else {
            two()
        }
    }
}
