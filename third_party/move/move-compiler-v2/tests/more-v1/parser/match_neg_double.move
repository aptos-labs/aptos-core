module 0xc0ffee::m {
    fun double_neg(x: i8): i8 {
        match (x) {
            - -1 => 1,
            _ => 0,
        }
    }
}
