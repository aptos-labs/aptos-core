// Tests that decompiling `is` expressions on cross-module generic enums preserves
// the type instantiation. Each function exercises test_variant$S$V with a distinct
// type instantiation to ensure the inst parameter is correctly forwarded.

module 0x42::defs {
    public enum Result<T: copy + drop, E: copy + drop> has copy, drop {
        Ok { value: T },
        Err { error: E },
    }
}

module 0x42::consumer {
    use 0x42::defs::Result;

    // test_variant$Result$Ok on Result<u64, u8>
    fun is_ok_u64(r: &Result<u64, u8>): bool {
        r is Result::Ok
    }

    // test_variant$Result$Err on Result<u64, u8>
    fun is_err_u64(r: &Result<u64, u8>): bool {
        r is Result::Err
    }

    // test_variant$Result$Ok on a different instantiation Result<bool, u64>
    fun is_ok_bool(r: &Result<bool, u64>): bool {
        r is Result::Ok
    }
}
