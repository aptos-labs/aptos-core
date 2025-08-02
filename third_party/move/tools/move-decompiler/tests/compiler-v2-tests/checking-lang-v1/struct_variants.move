module 0x42::m {

    enum S { None, Some{x: u64} }

    fun f(s: S) {
        match (s) {
            None => {}
            Some{x: _} => {}
        }
    }
}
