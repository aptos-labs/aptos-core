// inference-output: file
// A member-only import does not bind its module qualifier. File-mode output
// must not let a same-named module alias retarget the repeated signature.
module 0x1::X {
    public struct S has copy, drop {}
}

module 0x2::X {
    public struct T has copy, drop {}
}

module 0x42::file_signature_alias {
    use 0x1::X;
    use 0x2::X::T;

    public struct Local has copy, drop {}

    public fun identity(value: T): T {
        let _same_named_module_is_used = X::S {};
        value
    }

    public fun keep_local(value: Local): Local {
        value
    }
}
