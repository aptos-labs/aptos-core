module 0x8675309::M {
    struct S<T> has copy, drop { s: T }
    fun t(s: signer): S<signer> {
        let x = S<signer> { s };
        *&x
    }
}
// check: READREF_RESOURCE_ERROR
