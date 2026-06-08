/// Parametric test showcase — Layer 1 REJECTED syntax.
///
/// This file intentionally contains invalid parametric test constructs.
/// Running `aptos move test` on this package shows the compiler diagnostics
/// emitted for each violation.
module showcase_err::errors {
    use std::signer;

    // ---------------------------------------------------------------------------
    // REJECTED: multiple #[test] in one bracket
    // A row bracket must contain exactly one #[test].
    // ---------------------------------------------------------------------------

    #[test(a = @0x1), test(a = @0x2)]
    fun multiple_tests_same_bracket(a: signer) {
        let _ = signer::address_of(&a);
    }

    // ---------------------------------------------------------------------------
    // REJECTED: unrelated sibling attribute in a row bracket
    // A row bracket may only contain #[test] and #[expected_failure].
    // ---------------------------------------------------------------------------

    #[test(addr = @0x1), deprecated]
    fun unrelated_sibling(addr: address) {
        let _ = addr;
    }

    // ---------------------------------------------------------------------------
    // REJECTED: unknown parameter name in assignment
    // ---------------------------------------------------------------------------

    #[test(real = @0x1, typo = @0x2)]
    fun unknown_argument(real: signer) {
        let _ = signer::address_of(&real);
    }

    // ---------------------------------------------------------------------------
    // REJECTED: duplicate parameter assignment in one row
    // ---------------------------------------------------------------------------

    #[test(addr = @0x1, addr = @0x2)]
    fun duplicate_argument(addr: signer) {
        let _ = signer::address_of(&addr);
    }

    // ---------------------------------------------------------------------------
    // REJECTED: top-level expected_failure on a multi-row function
    // Use row-local syntax instead.
    // ---------------------------------------------------------------------------

    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    #[expected_failure]
    fun toplevel_ef_on_multirow(addr: address) {
        let _ = addr;
    }

    // ---------------------------------------------------------------------------
    // REJECTED: missing argument — function has parameter b but row omits it
    // ---------------------------------------------------------------------------

    #[test(a = @0x1)]
    fun missing_argument(a: signer, b: address) {
        let _ = signer::address_of(&a);
        let _ = b;
    }
}
