module 0xc0ffee::missing_doc_module {
    /// Dummy.
    const DUMMY: u64 = 0;
}

/// This module has a doc comment.
module 0xc0ffee::documented_module {
    /// Dummy.
    const DUMMY: u64 = 0;
}

#[lint::skip(missing_doc_module)]
module 0xc0ffee::skipped_module {
    /// Dummy.
    const DUMMY: u64 = 0;
}
