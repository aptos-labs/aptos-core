module std::signer {
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
    ///
    /// `borrow_address` borrows this inner field
    native public fun borrow_address(s: &signer): &address;

    // Copies the address of the signer
    public fun address_of(s: &signer): address {
        *borrow_address(s)
    }

    /// Return true only if `s` is a transaction signer. This is a spec function only available in spec.
    spec native fun is_txn_signer(s: signer): bool;

    /// Return true only if `a` is a transaction signer address. This is a spec function only available in spec.
    spec native fun is_txn_signer_addr(a: address): bool;
}
