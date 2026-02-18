module 0xc0ffee::m {
    fun neg_bool(x: bool): bool {
        match (x) {
            -true => true,
            _ => false,
        }
    }
}
