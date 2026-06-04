// Valid row brackets are order-insensitive, assignments are reordered to match
// the function signature, and identical rows are accepted.
address 0x1 {
module M {
    #[expected_failure, test(addr = @0x1)]
    fun expected_failure_before_test(addr: signer) {
        let _ = addr;
        abort 1
    }

    #[test(b = @0x2, a = @0x1)]
    fun assignment_order_is_irrelevant(a: signer, b: address) {
        let _ = a;
        let _ = b;
    }

    #[test(addr = @0x1)]
    #[test(addr = @0x1)]
    fun identical_rows_are_allowed(addr: signer) {
        let _ = addr;
    }
}
}
