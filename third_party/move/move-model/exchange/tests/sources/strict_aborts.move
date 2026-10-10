module 0x42::strict_aborts {
    fun f(): u64 { 0 }

    spec f {
        pragma aborts_if_is_strict = true;
    }
}
