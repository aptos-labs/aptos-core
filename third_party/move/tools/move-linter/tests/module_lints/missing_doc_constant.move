/// Test module for missing_doc_constant lint.
module 0xc0ffee::missing_doc_constant {
    const MY_CONSTANT: u64 = 42;

    const SOME_VALUE: u64 = 100;

    /// This constant has a doc comment.
    const DOCUMENTED_CONSTANT: u64 = 1;

    /// Error constants are handled by missing_doc_error_constant lint.
    const ENOT_FOUND: u64 = 1;

    /// Error constants are handled by missing_doc_error_constant lint.
    const E_NOT_FOUND: u64 = 2;

    #[lint::skip(missing_doc_constant)]
    const SKIPPED_CONSTANT: u64 = 99;
}
