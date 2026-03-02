/// Test module for error_message_too_long lint.
module 0xc0ffee::error_message_too_long {
    /// Short error message.
    const ESHORT: u64 = 1;

    /// This is a very long error message that exceeds the 128 byte limit. It contains a lot of unnecessary detail that makes the error description overly verbose and hard to read quickly at a glance.
    const ETOO_LONG: u64 = 2;

    /// This is a very long doc comment on a non-error constant. It exceeds the byte limit but that should be fine since this is not an error constant at all, so length does not matter here.
    const MY_LONG_CONSTANT: u64 = 42;

    #[lint::skip(error_message_too_long)]
    /// This is another very long error message that exceeds the 128 byte limit and contains too much verbosity. It should not trigger a warning because of the skip annotation though.
    const ESKIPPED_LONG: u64 = 99;
}
