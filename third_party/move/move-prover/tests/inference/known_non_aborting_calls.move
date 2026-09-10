module 0x42::known_non_aborting_calls {
    use std::string;

    fun constant(): u64 {
        7
    }

    fun calls_inferred_no_abort(): u64 {
        constant()
    }

    fun constructs_empty_string(): bool {
        string::utf8(b"").length() == 0
    }
}
