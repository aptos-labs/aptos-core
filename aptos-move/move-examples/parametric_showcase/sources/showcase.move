/// Parametric test showcase — Layer 1 accepted syntax.
///
/// Each #[test(...)] bracket is an independent test invocation of the same function.
/// The compiler emits one TestCase per row; each row executes with its own arguments.
module showcase::showcase {
    use std::signer;

    // ---------------------------------------------------------------------------
    // Module logic under test
    // ---------------------------------------------------------------------------

    const EBLACKLISTED: u64 = 1;
    const BLACKLISTED: address = @0xBAD;

    public fun blacklisted(addr: address): bool {
        addr == BLACKLISTED
    }

    public fun owner_of(account: &signer): address {
        signer::address_of(account)
    }

    // ---------------------------------------------------------------------------
    // Multi-row, signer parameter
    // Each row runs the body independently with a different signer.
    // ---------------------------------------------------------------------------

    #[test(account = @0x1)]
    #[test(account = @0x2)]
    #[test(account = @0x3)]
    fun different_signers_are_not_blacklisted(account: signer) {
        assert!(!blacklisted(owner_of(&account)), 0);
    }

    // ---------------------------------------------------------------------------
    // Multi-row, address parameter
    // ---------------------------------------------------------------------------

    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    #[test(addr = @0x3)]
    fun different_addresses_are_not_blacklisted(addr: address) {
        assert!(!blacklisted(addr), 0);
    }

    // ---------------------------------------------------------------------------
    // Multi-row, row-local expected_failure on some rows
    // @row0 expects success; @row1 expects the abort.
    // ---------------------------------------------------------------------------

    #[test(addr = @0x1)]
    #[test(addr = @0xBAD), expected_failure(abort_code = EBLACKLISTED)]
    fun blacklist_rejects_bad_address(addr: address) {
        assert!(!blacklisted(addr), EBLACKLISTED);
    }

    // ---------------------------------------------------------------------------
    // Multi-row, every row expects failure
    // ---------------------------------------------------------------------------

    #[test(addr = @0xBAD), expected_failure(abort_code = EBLACKLISTED)]
    #[test(addr = @0xBAD), expected_failure(abort_code = EBLACKLISTED)]
    fun blacklisted_always_aborts(addr: address) {
        assert!(!blacklisted(addr), EBLACKLISTED);
    }

    // ---------------------------------------------------------------------------
    // Two parameters — assignment order in the bracket does NOT matter.
    // Execution always uses function-signature order (a first, then b).
    // ---------------------------------------------------------------------------

    #[test(b = @0x2, a = @0x1)]
    #[test(a = @0x3, b = @0x4)]
    fun order_insensitive_assignments(a: signer, b: address) {
        assert!(owner_of(&a) != b, 0);
    }

    // ---------------------------------------------------------------------------
    // Single-row — backward-compatible, identity unchanged (no @row suffix).
    // ---------------------------------------------------------------------------

    #[test(account = @0x1)]
    fun single_row_signer(account: signer) {
        assert!(!blacklisted(owner_of(&account)), 0);
    }

    // ---------------------------------------------------------------------------
    // Single-row, row-local expected_failure (inline bracket syntax).
    // ---------------------------------------------------------------------------

    #[test(addr = @0xBAD), expected_failure(abort_code = EBLACKLISTED)]
    fun single_row_inline_failure(addr: address) {
        assert!(!blacklisted(addr), EBLACKLISTED);
    }

    // ---------------------------------------------------------------------------
    // Single-row, legacy top-level expected_failure (separate bracket syntax).
    // ---------------------------------------------------------------------------

    #[test(addr = @0xBAD)]
    #[expected_failure(abort_code = EBLACKLISTED)]
    fun single_row_legacy_failure(addr: address) {
        assert!(!blacklisted(addr), EBLACKLISTED);
    }

    // ---------------------------------------------------------------------------
    // Zero-argument function — single row, no parameters.
    // ---------------------------------------------------------------------------

    #[test]
    fun zero_arg_row() {
        assert!(!blacklisted(@0x1), 0);
    }
}
