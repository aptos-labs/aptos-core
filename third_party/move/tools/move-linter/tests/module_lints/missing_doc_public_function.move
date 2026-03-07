/// Test module for missing_doc_public_function lint.
module 0xc0ffee::missing_doc_public_function {
    /// Documented error constant.
    const EDUMMY: u64 = 0;

    /// Documented constant.
    const DUMMY: u64 = 0;

    public fun no_doc_public() {}

    entry fun no_doc_entry() {}

    /// This function has a doc comment.
    public fun has_doc_public() {}

    /// This entry function has a doc comment.
    entry fun has_doc_entry() {}

    fun private_no_doc() {}

    public(friend) fun friend_no_doc() {}

    #[lint::skip(missing_doc_public_function)]
    public fun skipped_no_doc() {}
}
