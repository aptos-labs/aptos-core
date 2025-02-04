module std::signer {
    use std::error;

    /// signer is a builtin move type that represents an address that has been verfied by the VM.
    ///
    /// VM Runtime representation is equivalent to following:
    /// ```
    /// enum signer has drop {
    ///     Master { account: address },
    ///     Permissioned { account: address, permissions_address: address },
    /// }
    /// ```
    ///
    /// for bcs serialization:
    ///
    /// ```
    /// struct signer has drop {
    ///     account: address,
    /// }
    /// ```
    /// ^ The discrepency is needed to maintain backwards compatibility of signer serialization
    /// semantics.

    /// Access address of a permissioned signer;
    const ENOT_MASTER_SIGNER: u64 = 1;

    /// `borrow_address` borrows this inner field, abort if `s` is a permissioned signer.
    native public fun borrow_address(s: &signer): &address;

    /// `borrow_address_unpermissioned` borrows this inner field, without checking if `s` is a permissioned signer.
    native fun borrow_address_unpermissioned(s: &signer): &address;

    // Copies the address of the signer, abort if `s` is a permissioned signer.
    public fun address_of(s: &signer): address {
        *borrow_address(s)
    }

    // Copies the address of the signer, without checking if `s` is a permissioned signer.
    public fun address_of_unpermissioned(s: &signer): address {
        *borrow_address_unpermissioned(s)
    }

    native fun is_permissioned_signer(s: &signer): bool;

    /// Return true only if `s` is a transaction signer. This is a spec function only available in spec.
    spec native fun is_txn_signer(s: signer): bool;

    /// Return true only if `a` is a transaction signer address. This is a spec function only available in spec.
    spec native fun is_txn_signer_addr(a: address): bool;
}
