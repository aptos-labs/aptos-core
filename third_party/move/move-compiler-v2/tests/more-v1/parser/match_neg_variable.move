module 0xc0ffee::m {
    fun neg_var(x: i8): i8 {
        match (x) {
            -x => x,
            _ => 0,
        }
    }
}
