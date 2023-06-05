module aptos_framework::transaction_context {
    /// Return the transaction hash of the current transaction
    public native fun get_txn_hash(): vector<u8>;

    /// Return a universally unique identifier
    public native fun create_uuid(): address;

    /// Return the script hash of the current entry function.
    public native fun get_script_hash(): vector<u8>;
}
