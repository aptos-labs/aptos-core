module aptos_framework::transaction_context {
    /// Return a universally unique identifier
    public native fun create_uuid(): address;

    /// Return the script hash of the current entry function.
    public native fun get_script_hash(): vector<u8>;
}
