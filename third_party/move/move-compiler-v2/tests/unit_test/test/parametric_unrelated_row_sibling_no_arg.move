// A no-argument row bracket also rejects unrelated sibling attributes.
address 0x1 {
module M {
    #[test, deprecated]
    fun unrelated_row_sibling_no_arg() {}
}
}
