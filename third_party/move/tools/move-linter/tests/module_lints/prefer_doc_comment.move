/// Test module for prefer_doc_comment lint.
module 0xc0ffee::prefer_doc_comment {
    // This should be a doc comment on a public function.
    public fun regular_comment_func() {}

    /// This is a proper doc comment.
    public fun doc_comment_func() {}

    // This should be a doc comment on a constant.
    const MY_CONSTANT: u64 = 42;

    /// Properly documented constant.
    const DOCUMENTED: u64 = 1;

    // This should be a doc comment on a struct.
    struct MyStruct has copy, drop {
        value: u64,
    }

    /// Properly documented struct.
    struct DocumentedStruct has copy, drop {
        value: u64,
    }

    // This should be a doc comment on a private function.
    fun private_func() {}

    #[lint::skip(prefer_doc_comment)]
    // Not flagged: skipped via lint::skip.
    public fun skipped_func() {}
}
