/// Test module for missing_doc_error_constant lint.
module 0xc0ffee::missing_doc_error_constant {
    const ENOT_FOUND: u64 = 1;

    const E_PERMISSION_DENIED: u64 = 2;

    /// The requested resource was not found.
    const ERESOURCE_NOT_FOUND: u64 = 3;

    /// Permission was denied for this operation.
    const E_UNAUTHORIZED: u64 = 4;

    /// Not an error constant.
    const MY_CONSTANT: u64 = 42;

    #[lint::skip(missing_doc_error_constant)]
    const ESKIPPED: u64 = 99;
}
