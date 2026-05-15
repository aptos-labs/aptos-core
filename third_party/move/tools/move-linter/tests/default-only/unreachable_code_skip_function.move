// Function-level `#[lint::skip(unreachable_code)]` should suppress the
// unreachable_code lint on `skipped`, but the warning still fires on `not_skipped`.
module 0xc0ffee::m {
    #[lint::skip(unreachable_code)]
    public fun skipped(): u64 {
        abort 0;
        42
    }

    public fun not_skipped(): u64 {
        abort 0;
        42
    }
}
