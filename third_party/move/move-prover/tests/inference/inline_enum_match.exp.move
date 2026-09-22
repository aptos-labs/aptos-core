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
        pragma opaque = true;
        ensures [inferred] (value is None) && 0 == (match (value) {
            Optional::None<u64>{} => 0,
            Optional::Some<u64>{value: value} => value,
        }) ==> result == 0;
        ensures [inferred] (value is Some) && value.value == (match (value) {
            Optional::None<u64>{} => 0,
            Optional::Some<u64>{value: value} => value,
        }) ==> result == value.value;
        aborts_if [inferred] false;
    }

    fun target(value: Optional<u64>): u64 {
        extract(value)
    }

    spec target {
        pragma opaque = true;
        ensures [inferred] (value is Some) ==> result == value.value;
        aborts_if [inferred] value is None;
    }
}
/*
Verification: Succeeded.
*/
