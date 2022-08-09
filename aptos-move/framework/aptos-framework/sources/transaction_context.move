module aptos_framework::transaction_context {
    /// Return the script hash of the current script function.
    public native fun get_script_hash(): vector<u8>;
}
