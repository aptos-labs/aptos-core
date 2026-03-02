/// Test module for error_constant_naming lint.
module 0xc0ffee::error_constant_naming {
    /// Good: follows EERROR_NAME convention.
    const ENOT_FOUND: u64 = 1;

    /// Good: follows E_ERROR_NAME convention.
    const E_NOT_FOUND: u64 = 2;

    /// Good: follows EERROR_NAME convention.
    const EPERMISSION_DENIED: u64 = 3;

    /// Not an error constant (doesn't start with E).
    const MY_CONSTANT: u64 = 42;

    /// Bad: lowercase after E does not follow convention.
    const Efoo: u64 = 100;

    #[lint::skip(error_constant_naming)]
    /// Skipped bad naming.
    const Ebar: u64 = 101;
}
