module 0x42::inline_enum_match {
    enum Optional<T> {
        None,
        Some { value: T },
    }

    inline fun extract(value: Optional<u64>): u64 {
        match (value) {
            Optional::None => abort 1,
            Optional::Some { value } => value,
        }
    }

    fun value_or_zero(value: Optional<u64>): u64 {
        match (value) {
            Optional::None => 0,
            Optional::Some { value } => value,
        }
    }

    spec fun spec_value_or_zero(value: Optional<u64>): u64 {
        match (value) {
            Optional::None => 0,
            Optional::Some { value } => value,
        }
    }

    spec value_or_zero {
        ensures result == spec_value_or_zero(value);
    }

    fun target(value: Optional<u64>): u64 {
        extract(value)
    }

    spec target {}
}
