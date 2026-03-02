/// Test module for missing_doc_struct lint.
module 0xc0ffee::missing_doc_struct {
    struct MyStruct has copy, drop {
        value: u64,
    }

    enum MyEnum has copy, drop {
        A,
        B(u64),
    }

    /// This struct has a doc comment.
    struct DocumentedStruct has copy, drop {
        value: u64,
    }

    /// This enum has a doc comment.
    enum DocumentedEnum has copy, drop {
        X,
        Y(u64),
    }

    #[lint::skip(missing_doc_struct)]
    struct SkippedStruct has copy, drop {
        value: u64,
    }
}
