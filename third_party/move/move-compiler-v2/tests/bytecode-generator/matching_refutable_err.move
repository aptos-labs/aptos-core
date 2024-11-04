module 0x815::m {

    enum E {
        None,
        Some(u64)
    }

    fun t(self: E): u64 {
        // We currently allow matching refutable patterns with let
        let Some(x) = self;
        x
    }
}
